//! Bounded, caller-driven Playwright Server client state.
//!
//! This crate has no socket, runtime, browser, or CDP dependency. A caller
//! supplies a text transport and feeds received frames to [`ClientActor`].
//! It deliberately does not expose a usable remote `Browser`.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt::{self, Display};
use std::io::{self, Write};

use serde::Serialize;
use serde_json::{Map, Value};

/// Protocol ceilings are independent of a transport's own frame ceiling.
pub const MAX_FRAME_BYTES: usize = 256 * 1024;
pub const MAX_OUTBOUND_BYTES: usize = 64 * 1024;
pub const MAX_PENDING: usize = 1024;
pub const MAX_OBJECTS: usize = 16_384;
pub const MAX_GUID_BYTES: usize = 512;
pub const MAX_METHOD_BYTES: usize = 256;
pub const MAX_CLOSE_REASON_BYTES: usize = 4096;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct CommandId(u64);

impl CommandId {
    pub const fn get(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProtocolViolation {
    FrameTooLarge,
    InvalidJson,
    InvalidEnvelope,
    InvalidId,
    UnknownResponse(CommandId),
    InvalidGuid,
    UnknownGuid(String),
    DuplicateGuid(String),
    InvalidMethod,
    InvalidParams,
    InvalidReply,
    InvalidError,
    RootMutation,
    AdoptionCycle,
    ObjectLimit,
}

impl Display for ProtocolViolation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Playwright Server protocol violation: {self:?}")
    }
}

impl Error for ProtocolViolation {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ClientError {
    Closed,
    InvalidGuid,
    UnknownGuid(String),
    InvalidMethod,
    InvalidParams,
    InvalidMetadata,
    PendingLimit,
    CommandIdExhausted,
    OutboundTooLarge,
}

impl Display for ClientError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Playwright Server client error: {self:?}")
    }
}

impl Error for ClientError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CloseReason {
    Remote(String),
    Protocol(ProtocolViolation),
    Transport,
    ResourceLimit,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PendingCommand {
    pub id: CommandId,
    pub guid: String,
    pub method: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CloseReport {
    pub reason: CloseReason,
    pub failed: Vec<PendingCommand>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum RemoteError {
    Exception {
        name: String,
        message: String,
        stack: Option<String>,
        log: Option<Value>,
    },
    Value {
        serialized: Value,
        log: Option<Value>,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub enum ReplyOutcome {
    Result(Value),
    Error(RemoteError),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Ingress {
    Reply {
        pending: PendingCommand,
        outcome: ReplyOutcome,
    },
    Created {
        parent: String,
        guid: String,
        object_type: String,
        initializer: Value,
    },
    Adopted {
        parent: String,
        guid: String,
    },
    Disposed {
        guids: Vec<String>,
        reason: Option<String>,
    },
    Event {
        guid: String,
        method: String,
        params: Value,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct ObjectEntry {
    pub object_type: String,
    pub parent: Option<String>,
    children: BTreeSet<String>,
}

impl ObjectEntry {
    pub fn children(&self) -> &BTreeSet<String> {
        &self.children
    }
}

/// A validated wire frame. The optional metadata is retained for future
/// per-command tracing, but this protocol core does not interpret it.
#[derive(Debug, Serialize)]
struct WireCommand<'a> {
    id: u64,
    guid: &'a str,
    method: &'a str,
    params: &'a Value,
    metadata: &'a Value,
}

struct BoundedFrame {
    bytes: Vec<u8>,
}

impl Write for BoundedFrame {
    fn write(&mut self, input: &[u8]) -> io::Result<usize> {
        if input.len() > MAX_OUTBOUND_BYTES.saturating_sub(self.bytes.len()) {
            return Err(io::Error::other(
                "Playwright Server outbound frame exceeds limit",
            ));
        }
        self.bytes.extend_from_slice(input);
        Ok(input.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum WireInbound {
    Reply {
        id: CommandId,
        outcome: ReplyOutcome,
    },
    Event {
        guid: String,
        method: String,
        params: Value,
    },
}

/// Decode a single complete Playwright Server text message.
pub fn decode_inbound(text: &str) -> Result<WireInbound, ProtocolViolation> {
    if text.len() > MAX_FRAME_BYTES {
        return Err(ProtocolViolation::FrameTooLarge);
    }
    let value: Value = serde_json::from_str(text).map_err(|_| ProtocolViolation::InvalidJson)?;
    let object = value
        .as_object()
        .ok_or(ProtocolViolation::InvalidEnvelope)?;
    if let Some(raw_id) = object.get("id") {
        if object.contains_key("guid") || object.contains_key("method") {
            return Err(ProtocolViolation::InvalidEnvelope);
        }
        let id = raw_id
            .as_u64()
            .filter(|id| *id > 0)
            .map(CommandId)
            .ok_or(ProtocolViolation::InvalidId)?;
        let outcome = match (object.get("result"), object.get("error")) {
            (None, None) => ReplyOutcome::Result(Value::Null),
            (Some(result), None) => ReplyOutcome::Result(result.clone()),
            (None, Some(error)) => ReplyOutcome::Error(decode_remote_error(error, object)?),
            (Some(_), Some(_)) => return Err(ProtocolViolation::InvalidReply),
        };
        return Ok(WireInbound::Reply { id, outcome });
    }
    if object.contains_key("result") || object.contains_key("error") {
        return Err(ProtocolViolation::InvalidId);
    }
    let guid = object
        .get("guid")
        .and_then(Value::as_str)
        .ok_or(ProtocolViolation::InvalidGuid)?;
    validate_guid(guid, true)?;
    let method = object
        .get("method")
        .and_then(Value::as_str)
        .ok_or(ProtocolViolation::InvalidMethod)?;
    validate_method(method)?;
    let params = object.get("params").cloned().unwrap_or(Value::Null);
    Ok(WireInbound::Event {
        guid: guid.to_owned(),
        method: method.to_owned(),
        params,
    })
}

fn decode_remote_error(
    value: &Value,
    envelope: &Map<String, Value>,
) -> Result<RemoteError, ProtocolViolation> {
    // Official `serializeError` nests the serialized exception under `error`.
    let wrapper = value.as_object().ok_or(ProtocolViolation::InvalidError)?;
    if let Some(serialized) = wrapper.get("value") {
        if wrapper.contains_key("error") {
            return Err(ProtocolViolation::InvalidError);
        }
        return Ok(RemoteError::Value {
            serialized: serialized.clone(),
            log: envelope.get("log").cloned(),
        });
    }
    let error = wrapper
        .get("error")
        .and_then(Value::as_object)
        .ok_or(ProtocolViolation::InvalidError)?;
    let name = error
        .get("name")
        .and_then(Value::as_str)
        .ok_or(ProtocolViolation::InvalidError)?;
    let message = error
        .get("message")
        .and_then(Value::as_str)
        .ok_or(ProtocolViolation::InvalidError)?;
    if name.is_empty() || name.len() > MAX_METHOD_BYTES || message.len() > MAX_FRAME_BYTES {
        return Err(ProtocolViolation::InvalidError);
    }
    let stack = match error.get("stack") {
        None => None,
        Some(Value::String(stack)) if stack.len() <= MAX_FRAME_BYTES => Some(stack.clone()),
        _ => return Err(ProtocolViolation::InvalidError),
    };
    Ok(RemoteError::Exception {
        name: name.to_owned(),
        message: message.to_owned(),
        stack,
        log: envelope.get("log").cloned(),
    })
}

fn validate_guid(guid: &str, allow_root: bool) -> Result<(), ProtocolViolation> {
    if (!allow_root && guid.is_empty())
        || guid.len() > MAX_GUID_BYTES
        || guid.chars().any(char::is_control)
    {
        return Err(ProtocolViolation::InvalidGuid);
    }
    Ok(())
}

fn validate_method(method: &str) -> Result<(), ProtocolViolation> {
    if method.is_empty() || method.len() > MAX_METHOD_BYTES || method.chars().any(char::is_control)
    {
        return Err(ProtocolViolation::InvalidMethod);
    }
    Ok(())
}

/// State machine for IDs, callbacks and the server-owned GUID graph.
pub struct ClientCore {
    last_id: u64,
    pending: BTreeMap<CommandId, PendingCommand>,
    objects: BTreeMap<String, ObjectEntry>,
    closed: Option<CloseReason>,
}

impl Default for ClientCore {
    fn default() -> Self {
        Self::new()
    }
}

impl ClientCore {
    pub fn new() -> Self {
        let root = ObjectEntry {
            object_type: "Root".to_owned(),
            parent: None,
            children: BTreeSet::new(),
        };
        Self {
            last_id: 0,
            pending: BTreeMap::new(),
            objects: BTreeMap::from([(String::new(), root)]),
            closed: None,
        }
    }

    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    pub fn object_count(&self) -> usize {
        self.objects.len()
    }

    pub fn object(&self, guid: &str) -> Option<&ObjectEntry> {
        self.objects.get(guid)
    }

    pub fn close_reason(&self) -> Option<&CloseReason> {
        self.closed.as_ref()
    }

    pub fn last_command_id(&self) -> u64 {
        self.last_id
    }

    /// Prepare a command for an injected transport. IDs are never reused.
    pub fn begin_command(
        &mut self,
        guid: &str,
        method: &str,
        params: &Value,
        metadata: &Value,
    ) -> Result<(CommandId, String), ClientError> {
        if self.closed.is_some() {
            return Err(ClientError::Closed);
        }
        validate_guid(guid, true).map_err(|_| ClientError::InvalidGuid)?;
        if !self.objects.contains_key(guid) {
            return Err(ClientError::UnknownGuid(guid.to_owned()));
        }
        validate_method(method).map_err(|_| ClientError::InvalidMethod)?;
        if !params.is_object() {
            return Err(ClientError::InvalidParams);
        }
        if !metadata.is_object() {
            return Err(ClientError::InvalidMetadata);
        }
        if self.pending.len() >= MAX_PENDING {
            return Err(ClientError::PendingLimit);
        }
        let next = self
            .last_id
            .checked_add(1)
            .ok_or(ClientError::CommandIdExhausted)?;
        let wire = WireCommand {
            id: next,
            guid,
            method,
            params,
            metadata,
        };
        let mut output = BoundedFrame { bytes: Vec::new() };
        serde_json::to_writer(&mut output, &wire).map_err(|_| ClientError::OutboundTooLarge)?;
        let frame = String::from_utf8(output.bytes).map_err(|_| ClientError::OutboundTooLarge)?;
        self.last_id = next;
        let id = CommandId(next);
        self.pending.insert(
            id,
            PendingCommand {
                id,
                guid: guid.to_owned(),
                method: method.to_owned(),
            },
        );
        Ok((id, frame))
    }

    /// A violation terminates the core so no callback can be stranded.
    pub fn ingest(&mut self, text: &str) -> Result<Ingress, ReceiveFailure> {
        if self.closed.is_some() {
            return Err(ReceiveFailure::Closed);
        }
        match self.decode_and_apply(text) {
            Ok(event) => Ok(event),
            Err(violation) => {
                let report = self.close(CloseReason::Protocol(violation.clone()));
                Err(ReceiveFailure::Protocol { violation, report })
            }
        }
    }

    fn decode_and_apply(&mut self, text: &str) -> Result<Ingress, ProtocolViolation> {
        match decode_inbound(text)? {
            WireInbound::Reply { id, outcome } => {
                let pending = self
                    .pending
                    .remove(&id)
                    .ok_or(ProtocolViolation::UnknownResponse(id))?;
                Ok(Ingress::Reply { pending, outcome })
            }
            WireInbound::Event {
                guid,
                method,
                params,
            } => self.apply_event(guid, method, params),
        }
    }

    fn apply_event(
        &mut self,
        guid: String,
        method: String,
        params: Value,
    ) -> Result<Ingress, ProtocolViolation> {
        if !self.objects.contains_key(&guid) {
            return Err(ProtocolViolation::UnknownGuid(guid));
        }
        match method.as_str() {
            "__create__" => self.apply_create(guid, params),
            "__adopt__" => self.apply_adopt(guid, params),
            "__dispose__" => self.apply_dispose(guid, params),
            _ => Ok(Ingress::Event {
                guid,
                method,
                params,
            }),
        }
    }

    fn apply_create(
        &mut self,
        parent: String,
        params: Value,
    ) -> Result<Ingress, ProtocolViolation> {
        let params = params.as_object().ok_or(ProtocolViolation::InvalidParams)?;
        let guid = field_str(params, "guid")?;
        validate_guid(guid, false)?;
        if self.objects.contains_key(guid) {
            return Err(ProtocolViolation::DuplicateGuid(guid.to_owned()));
        }
        if self.objects.len() >= MAX_OBJECTS {
            return Err(ProtocolViolation::ObjectLimit);
        }
        let object_type = field_str(params, "type")?;
        validate_method(object_type)?;
        let initializer = params
            .get("initializer")
            .filter(|value| value.is_object())
            .ok_or(ProtocolViolation::InvalidParams)?
            .clone();
        self.objects.insert(
            guid.to_owned(),
            ObjectEntry {
                object_type: object_type.to_owned(),
                parent: Some(parent.clone()),
                children: BTreeSet::new(),
            },
        );
        self.objects
            .get_mut(&parent)
            .expect("parent validated before event dispatch")
            .children
            .insert(guid.to_owned());
        Ok(Ingress::Created {
            parent,
            guid: guid.to_owned(),
            object_type: object_type.to_owned(),
            initializer,
        })
    }

    fn apply_adopt(&mut self, parent: String, params: Value) -> Result<Ingress, ProtocolViolation> {
        let params = params.as_object().ok_or(ProtocolViolation::InvalidParams)?;
        let guid = field_str(params, "guid")?;
        validate_guid(guid, false)?;
        let child = self
            .objects
            .get(guid)
            .ok_or_else(|| ProtocolViolation::UnknownGuid(guid.to_owned()))?;
        let old_parent = child
            .parent
            .clone()
            .ok_or(ProtocolViolation::RootMutation)?;
        let mut ancestor = Some(parent.as_str());
        while let Some(current) = ancestor {
            if current == guid {
                return Err(ProtocolViolation::AdoptionCycle);
            }
            ancestor = self
                .objects
                .get(current)
                .and_then(|entry| entry.parent.as_deref());
        }
        self.objects
            .get_mut(&old_parent)
            .expect("adopted child's parent exists")
            .children
            .remove(guid);
        self.objects
            .get_mut(&parent)
            .expect("adopting parent validated")
            .children
            .insert(guid.to_owned());
        self.objects
            .get_mut(guid)
            .expect("adopted child validated")
            .parent = Some(parent.clone());
        Ok(Ingress::Adopted {
            parent,
            guid: guid.to_owned(),
        })
    }

    fn apply_dispose(&mut self, guid: String, params: Value) -> Result<Ingress, ProtocolViolation> {
        if guid.is_empty() {
            return Err(ProtocolViolation::RootMutation);
        }
        let params = params.as_object().ok_or(ProtocolViolation::InvalidParams)?;
        let reason = match params.get("reason") {
            None | Some(Value::Null) => None,
            Some(Value::String(reason)) if reason.len() <= MAX_CLOSE_REASON_BYTES => {
                Some(reason.clone())
            }
            _ => return Err(ProtocolViolation::InvalidParams),
        };
        let parent = self
            .objects
            .get(&guid)
            .and_then(|entry| entry.parent.clone())
            .ok_or(ProtocolViolation::RootMutation)?;
        let mut queue = vec![guid.clone()];
        let mut index = 0;
        while index < queue.len() {
            let current = &queue[index];
            queue.extend(
                self.objects
                    .get(current)
                    .expect("queued object exists")
                    .children
                    .iter()
                    .cloned(),
            );
            index += 1;
        }
        self.objects
            .get_mut(&parent)
            .expect("disposed object's parent exists")
            .children
            .remove(&guid);
        for current in &queue {
            self.objects.remove(current);
        }
        Ok(Ingress::Disposed {
            guids: queue,
            reason,
        })
    }

    pub fn close(&mut self, reason: CloseReason) -> CloseReport {
        let reason = self.closed.get_or_insert(reason).clone();
        let failed = std::mem::take(&mut self.pending).into_values().collect();
        self.objects.clear();
        CloseReport { reason, failed }
    }

    pub fn remote_closed(&mut self, reason: &str) -> CloseReport {
        let reason = if reason.len() <= MAX_CLOSE_REASON_BYTES {
            reason.to_owned()
        } else {
            reason[..reason.floor_char_boundary(MAX_CLOSE_REASON_BYTES)].to_owned()
        };
        self.close(CloseReason::Remote(reason))
    }
}

fn field_str<'a>(params: &'a Map<String, Value>, key: &str) -> Result<&'a str, ProtocolViolation> {
    params
        .get(key)
        .and_then(Value::as_str)
        .ok_or(ProtocolViolation::InvalidParams)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReceiveFailure {
    Closed,
    Protocol {
        violation: ProtocolViolation,
        report: CloseReport,
    },
}

impl Display for ReceiveFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Playwright Server receive failed: {self:?}")
    }
}

impl Error for ReceiveFailure {}

/// Inject any transport implementation; no transport is selected by default.
pub trait TextTransport {
    type Error;

    fn send_text(&mut self, frame: &str) -> Result<(), Self::Error>;
}

#[derive(Debug)]
pub enum SendFailure<E> {
    Client(ClientError),
    Transport { error: E, report: CloseReport },
}

/// Small orchestration shell around the state machine and injected transport.
pub struct ClientActor<T> {
    core: ClientCore,
    transport: T,
}

impl<T: TextTransport> ClientActor<T> {
    pub fn new(transport: T) -> Self {
        Self {
            core: ClientCore::new(),
            transport,
        }
    }

    pub fn core(&self) -> &ClientCore {
        &self.core
    }

    pub fn transport(&self) -> &T {
        &self.transport
    }

    pub fn send_command(
        &mut self,
        guid: &str,
        method: &str,
        params: &Value,
        metadata: &Value,
    ) -> Result<CommandId, SendFailure<T::Error>> {
        let (id, frame) = self
            .core
            .begin_command(guid, method, params, metadata)
            .map_err(SendFailure::Client)?;
        if let Err(error) = self.transport.send_text(&frame) {
            let report = self.core.close(CloseReason::Transport);
            return Err(SendFailure::Transport { error, report });
        }
        Ok(id)
    }

    pub fn receive_text(&mut self, frame: &str) -> Result<Ingress, ReceiveFailure> {
        self.core.ingest(frame)
    }

    pub fn remote_closed(&mut self, reason: &str) -> CloseReport {
        self.core.remote_closed(reason)
    }
}

#[cfg(test)]
mod tests;
