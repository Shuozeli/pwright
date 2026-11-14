//! Optional Page binding bridge. Chromium installation, call routing, bounded
//! pending ownership and result delivery live in Rust; the language package
//! retains only the user callback and Playwright-shaped source wrappers.

use std::error::Error;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use pwright_backend_cdp_topology::CdpTopology;
use pwright_cdp::{CdpConnection, CdpEvent, generated::page};
use pwright_core::{
    ObjectId, ProductionResourceLedger, ResourceClass, ResourceLease, ResourceRequirements,
    ResourceScope,
};
use pwright_plugin_chromium_frame_observer::{ChromiumFrameEventObserver, FrameObservationResult};
use pwright_plugin_chromium_session_initialization::{
    ChromiumSessionInitializationFuture, ChromiumSessionInitializationTarget,
    ChromiumSessionInitializer,
};
use serde::Deserialize;
use serde_json::Value;
use tokio::sync::{Notify, Semaphore, mpsc};
use tokio_util::sync::CancellationToken;

use crate::{
    HostError, HostSession,
    evaluation::{EvaluateParams, EvaluationProperty, EvaluationValue},
    handles::{EvaluateHandleParams, OwnedHandleArgument, PendingHandle},
};

const SCOPE: ResourceScope = ResourceScope::extension(137, 1);
const MAX_BINDINGS: usize = 64;
const MAX_PENDING_CALLS: usize = 64;
const MAX_EARLY_CALLS: usize = 16;
const MAX_REJECTION_TASKS: usize = 16;
const MAX_REJECTION_TASK_BYTES: usize = 4096;
const MAX_BINDING_SESSIONS: usize = 128;
const MAX_BINDING_NAME_BYTES: usize = 256;
const MAX_SESSION_ID_BYTES: usize = 256;
const MAX_SCRIPT_IDENTIFIER_BYTES: usize = 256;
const MAX_ARGUMENT_BYTES: usize = 64 * 1024;
const MAX_BINDING_PAYLOAD_BYTES: usize = MAX_ARGUMENT_BYTES + MAX_BINDING_NAME_BYTES + 512;
const BRIDGE_NAME: &str = "__pwright_binding__";
const BRIDGE_SCHEMA: &str = "pwright.page-binding.v1";
const CONTROLLER_NAME: &str = "__pwright_binding_controller__";

// This source captures the built-ins it uses before page code can replace
// them. Value serialization is intentionally versioned separately from the
// native transport so handle/cycle support can be added without changing the
// CDP binding name or the Rust queue contract.
const CONTROLLER_SOURCE: &str = r#"(() => {
  const global = globalThis;
  if (global.__pwright_binding_controller__) return;
  const MapConstructor = Map;
  const PromiseConstructor = Promise;
  const arrayFrom = Array.from.bind(Array);
  const ErrorConstructor = Error;
  const DateConstructor = Date;
  const URLConstructor = URL;
  const RegExpConstructor = RegExp;
  const hasOwn = Function.prototype.call.bind(Object.prototype.hasOwnProperty);
  const objectToString = Function.prototype.call.bind(Object.prototype.toString);
  const objectKeys = Object.keys.bind(Object);
  const objectIs = Object.is.bind(Object);
  const arrayIsArray = Array.isArray.bind(Array);
  const arrayBufferIsView = ArrayBuffer.isView.bind(ArrayBuffer);
  const numberIsNaN = Number.isNaN.bind(Number);
  const stringify = JSON.stringify.bind(JSON);
  const fromCharCode = String.fromCharCode.bind(String);
  const encodeBase64 = global.btoa.bind(global);
  const callbacks = new MapConstructor();
  const bindings = new MapConstructor();
  const handles = new MapConstructor();
  const publications = new MapConstructor();
  const typedArrays = {
    i8: Int8Array,
    ui8: Uint8Array,
    ui8c: Uint8ClampedArray,
    i16: Int16Array,
    ui16: Uint16Array,
    i32: Int32Array,
    ui32: Uint32Array,
    f32: Float32Array,
    f64: Float64Array,
    ...(typeof BigInt64Array === 'undefined' ? {} : { bi64: BigInt64Array }),
    ...(typeof BigUint64Array === 'undefined' ? {} : { bui64: BigUint64Array }),
  };
  const typedArrayEntries = Object.entries(typedArrays);
  const WindowConstructor = typeof global.Window === 'function' ? global.Window : null;
  const DocumentConstructor = typeof global.Document === 'function' ? global.Document : null;
  const NodeConstructor = typeof global.Node === 'function' ? global.Node : null;
  const limits = () => ({ nodes: 0, entries: 0, strings: 0 });
  const utf8Length = value => {
    let bytes = 0;
    for (let index = 0; index < value.length; index++) {
      const code = value.charCodeAt(index);
      if (code < 0x80) bytes += 1;
      else if (code < 0x800) bytes += 2;
      else if (code >= 0xd800 && code <= 0xdbff && index + 1 < value.length &&
               value.charCodeAt(index + 1) >= 0xdc00 && value.charCodeAt(index + 1) <= 0xdfff) {
        bytes += 4;
        index += 1;
      } else bytes += 3;
    }
    return bytes;
  };
  const claimNode = (state, depth) => {
    if (depth > 64) throw new ErrorConstructor('binding value depth limit exceeded');
    if (++state.nodes > 4096) throw new ErrorConstructor('binding value node limit exceeded');
  };
  const claimEntry = state => {
    if (++state.entries > 4096) throw new ErrorConstructor('binding value entry limit exceeded');
  };
  const claimString = (state, value) => {
    state.strings += utf8Length(value);
    if (state.strings > 65536) throw new ErrorConstructor('binding value string-byte limit exceeded');
  };
  const isKind = (value, Constructor, tag) => {
    try {
      return value instanceof Constructor || objectToString(value) === tag;
    } catch (_) {
      return false;
    }
  };
  const bytesToBase64 = value => {
    const bytes = new Uint8Array(value.buffer, value.byteOffset, value.byteLength);
    let binary = '';
    for (let index = 0; index < bytes.length; index++) binary += fromCharCode(bytes[index]);
    return encodeBase64(binary);
  };
  const serialize = (value, visitor, depth) => {
    claimNode(visitor.limits, depth);
    if (typeof value === 'symbol' || typeof value === 'function' || value === undefined) return { v: 'undefined' };
    if (value === null) return { v: 'null' };
    if (numberIsNaN(value)) return { v: 'NaN' };
    if (value === Infinity) return { v: 'Infinity' };
    if (value === -Infinity) return { v: '-Infinity' };
    if (objectIs(value, -0)) return { v: '-0' };
    if (typeof value === 'boolean' || typeof value === 'number') return value;
    if (typeof value === 'string') {
      claimString(visitor.limits, value);
      return value;
    }
    if (typeof value === 'bigint') return { bi: value.toString() };
    if (value && typeof value === 'object') {
      if (WindowConstructor && value instanceof WindowConstructor) return 'ref: <Window>';
      if (DocumentConstructor && value instanceof DocumentConstructor) return 'ref: <Document>';
      if (NodeConstructor && value instanceof NodeConstructor) return 'ref: <Node>';
    }
    if (isKind(value, ErrorConstructor, '[object Error]')) {
      const name = String(value.name);
      const message = String(value.message);
      const stack = typeof value.stack === 'string' ? value.stack : `${name}: ${message}`;
      claimString(visitor.limits, name);
      claimString(visitor.limits, message);
      claimString(visitor.limits, stack);
      return { e: { n: name, m: message, s: stack } };
    }
    if (isKind(value, DateConstructor, '[object Date]')) return { d: value.toJSON() };
    if (isKind(value, URLConstructor, '[object URL]')) return { u: value.toString() };
    if (isKind(value, RegExpConstructor, '[object RegExp]')) return { r: { p: value.source, f: value.flags } };
    if (value && arrayBufferIsView(value) && !(value instanceof DataView)) {
      for (const entry of typedArrayEntries) {
        if (value instanceof entry[1]) return { ta: { b: bytesToBase64(value), k: entry[0] } };
      }
    }
    if (!value || typeof value !== 'object') return { v: 'undefined' };
    if (visitor.seen.has(value)) return { ref: visitor.seen.get(value) };
    const id = ++visitor.nextId;
    visitor.seen.set(value, id);
    if (arrayIsArray(value)) {
      const array = [];
      for (let index = 0; index < value.length; index++) {
        claimEntry(visitor.limits);
        array[index] = serialize(value[index], visitor, depth + 1);
      }
      return { a: array, id };
    }
    const object = [];
    const keys = objectKeys(value);
    for (let index = 0; index < keys.length; index++) {
      const key = keys[index];
      let item;
      try {
        item = value[key];
      } catch (_) {
        continue;
      }
      claimEntry(visitor.limits);
      claimString(visitor.limits, key);
      object[object.length] = {
        k: key,
        v: key === 'toJSON' && typeof item === 'function'
          ? { o: [], id: 0 }
          : serialize(item, visitor, depth + 1),
      };
    }
    if (!object.length) {
      try {
        if (typeof value.toJSON === 'function') return serialize(value.toJSON(), visitor, depth + 1);
      } catch (_) {}
    }
    return { o: object, id };
  };
  const controller = {
    add(name, needsHandle) {
      if (bindings.has(name)) return;
      let sequence = 0;
      const binding = (...args) => {
        if (callbacks.size >= 64)
          return new PromiseConstructor((_, reject) => reject(new ErrorConstructor('pending Page binding capacity exceeded (64)')));
        const seq = ++sequence;
        const key = `${name}:${seq}`;
        const promise = new PromiseConstructor((resolve, reject) => callbacks.set(key, { resolve, reject }));
        const publication = {};
        publication.promise = new PromiseConstructor(resolve => publication.resolve = resolve);
        publications.set(key, publication);
        try {
          let serializedArgs;
          if (needsHandle) {
            for (let index = 1; index < args.length; index++) {
              if (args[index] !== undefined)
                throw new ErrorConstructor(`exposeBindingHandle supports a single argument, ${args.length} received`);
            }
            handles.set(key, args[0]);
          } else {
            serializedArgs = [];
            for (let index = 0; index < args.length; index++)
              serializedArgs[index] = serialize(args[index], { seen: new MapConstructor(), nextId: 0, limits: limits() }, 0);
            if (utf8Length(stringify(serializedArgs)) > 65536)
              throw new ErrorConstructor('binding call arguments exceed byte limit');
          }
          const payload = { schema: 'pwright.page-binding.v1', name, seq };
          if (!needsHandle) payload.serializedArgs = serializedArgs;
          const encoded = stringify(payload);
          if (utf8Length(encoded) > 65536 + 256 + 512)
            throw new ErrorConstructor('binding payload exceeds byte limit');
          global.__pwright_binding__(encoded);
        } catch (error) {
          callbacks.delete(key);
          handles.delete(key);
          publications.delete(key);
          publication.resolve();
          throw error;
        }
        return promise;
      };
      bindings.set(name, binding);
      global[name] = binding;
    },
    remove(name) {
      const binding = bindings.get(name);
      if (binding && global[name] === binding) delete global[name];
      bindings.delete(name);
    },
    take(payload) {
      const key = `${payload.name}:${payload.seq}`;
      const handle = handles.get(key);
      handles.delete(key);
      return handle;
    },
    publish(payload) {
      const key = `${payload.name}:${payload.seq}`;
      const publication = publications.get(key);
      if (!publication) return;
      publications.delete(key);
      publication.resolve();
    },
    drainPublications() {
      return PromiseConstructor.all(arrayFrom(publications.values(), value => value.promise));
    },
    deliver(payload) {
      const key = `${payload.name}:${payload.seq}`;
      this.publish(payload);
      const callback = callbacks.get(key);
      if (!callback) return;
      callbacks.delete(key);
      handles.delete(key);
      if (hasOwn(payload, 'error')) callback.reject(payload.error);
      else callback.resolve(payload.result);
    },
  };
  Object.defineProperty(global, '__pwright_binding_controller__', {
    value: controller,
    configurable: true,
  });
})();"#;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BindingCall {
    pub id: u64,
    pub context: ObjectId,
    pub page: ObjectId,
    pub frame: ObjectId,
    pub name: String,
    pub arguments_json: String,
    pub needs_handle: bool,
}

#[derive(Clone)]
struct Registration {
    id: u64,
    scope: RegistrationScope,
    name: Box<str>,
    needs_handle: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RegistrationScope {
    Page {
        page: ObjectId,
        context: ObjectId,
    },
    #[cfg(any(feature = "context-bindings", test))]
    Context(ObjectId),
}

impl RegistrationScope {
    fn context(self) -> ObjectId {
        match self {
            Self::Page { context, .. } => context,
            #[cfg(any(feature = "context-bindings", test))]
            Self::Context(context) => context,
        }
    }

    fn applies_to(self, page: ObjectId, _context: ObjectId) -> bool {
        match self {
            Self::Page { page: expected, .. } => expected == page,
            #[cfg(any(feature = "context-bindings", test))]
            Self::Context(expected) => expected == _context,
        }
    }

    fn is_live(self, topology: &CdpTopology) -> bool {
        if topology.browser_context_route(self.context()).is_none() {
            return false;
        }
        match self {
            #[cfg(any(feature = "context-bindings", test))]
            Self::Context(_) => true,
            Self::Page { page, context } => topology.page_session_routes().any(|route| {
                route.object_id() == page && route.browser_context_object_id() == context
            }),
        }
    }
}

struct PendingCall {
    id: u64,
    context: ObjectId,
    page: ObjectId,
    frame: ObjectId,
    name: Box<str>,
    sequence: u64,
    needs_handle: bool,
    handle_taken: bool,
    published: bool,
    delivering: bool,
}

// A document-start script can call a binding before its main-world route is
// published. Retain only calls from a known, registered CDP session;
// neither an unowned session nor arbitrary page JavaScript can grow this queue.
struct EarlyCall {
    session: Arc<SessionInstallation>,
    session_id: Box<str>,
    execution_context_id: i64,
    context: ObjectId,
    page: ObjectId,
    registration_id: u64,
    payload: Box<str>,
}

#[derive(Clone)]
struct PendingOperation {
    frame: ObjectId,
    name: Box<str>,
    sequence: u64,
}

struct State {
    next_call: u64,
    next_registration: u64,
    registrations: Vec<Registration>,
    pending: Vec<PendingCall>,
    early: Vec<EarlyCall>,
    sessions: Vec<Arc<SessionInstallation>>,
}

struct SessionInstallation {
    session_id: Box<str>,
    page: ObjectId,
    context: ObjectId,
    initializing: AtomicBool,
    retired: AtomicBool,
    scripts: Arc<tokio::sync::Mutex<Vec<InstalledScript>>>,
}

struct InstalledScript {
    registration_id: u64,
    identifier: Box<str>,
}

impl State {
    #[cfg(test)]
    fn empty() -> Self {
        Self {
            next_call: 1,
            next_registration: 1,
            registrations: Vec::new(),
            pending: Vec::new(),
            early: Vec::new(),
            sessions: Vec::new(),
        }
    }
}

fn has_publication_sources(state: &State) -> bool {
    !state.registrations.is_empty() || !state.pending.is_empty() || !state.early.is_empty()
}

fn add_registration_to_state(
    state: &mut State,
    scope: RegistrationScope,
    name: &str,
    needs_handle: bool,
) -> Result<u64, HostError> {
    if name.is_empty() || name.len() > MAX_BINDING_NAME_BYTES {
        return Err(HostError::InvalidArgument(format!(
            "binding name must contain 1..={MAX_BINDING_NAME_BYTES} UTF-8 bytes"
        )));
    }
    for binding in &state.registrations {
        if binding.name.as_ref() != name || binding.scope.context() != scope.context() {
            continue;
        }
        let conflict = match (scope, binding.scope) {
            #[cfg(any(feature = "context-bindings", test))]
            (RegistrationScope::Context(_), RegistrationScope::Context(_)) => {
                Some(format!("Function \"{name}\" has been already registered"))
            }
            (
                RegistrationScope::Page {
                    page: requested, ..
                },
                RegistrationScope::Page {
                    page: registered, ..
                },
            ) if requested == registered => {
                Some(format!("Function \"{name}\" has been already registered"))
            }
            #[cfg(any(feature = "context-bindings", test))]
            (RegistrationScope::Page { .. }, RegistrationScope::Context(_)) => Some(format!(
                "Function \"{name}\" has been already registered in the browser context"
            )),
            #[cfg(any(feature = "context-bindings", test))]
            (RegistrationScope::Context(_), RegistrationScope::Page { .. }) => Some(format!(
                "Function \"{name}\" has been already registered in one of the pages"
            )),
            _ => None,
        };
        if let Some(message) = conflict {
            return Err(HostError::InvalidArgument(message));
        }
    }
    if state.registrations.len() == MAX_BINDINGS {
        return Err(HostError::ResourceExhausted(format!(
            "Page binding capacity exceeded ({MAX_BINDINGS})"
        )));
    }
    let id = state.next_registration;
    state.next_registration = id.checked_add(1).ok_or_else(|| {
        HostError::ResourceExhausted("binding registration ID space exhausted".into())
    })?;
    state.registrations.push(Registration {
        id,
        scope,
        name: name.to_owned().into_boxed_str(),
        needs_handle,
    });
    Ok(id)
}

fn remove_page_from_state(state: &mut State, page: ObjectId) {
    state
        .registrations
        .retain(|binding| !matches!(binding.scope, RegistrationScope::Page { page: owner, .. } if owner == page));
    state.pending.retain(|call| call.page != page);
    state.early.retain(|call| call.page != page);
    state.sessions.retain(|session| {
        let keep = session.page != page;
        if !keep {
            session.retired.store(true, Ordering::Release);
        }
        keep
    });
}

fn remove_context_from_state(state: &mut State, context: ObjectId) {
    state
        .registrations
        .retain(|binding| binding.scope.context() != context);
    state.pending.retain(|call| call.context != context);
    state.early.retain(|call| call.context != context);
    state.sessions.retain(|session| {
        let keep = session.context != context;
        if !keep {
            session.retired.store(true, Ordering::Release);
        }
        keep
    });
}

#[cfg(test)]
fn registrations_for_target(
    state: &State,
    page: ObjectId,
    context: ObjectId,
) -> Vec<(String, bool)> {
    state
        .registrations
        .iter()
        .filter(|binding| binding.scope.applies_to(page, context))
        .map(|binding| (binding.name.to_string(), binding.needs_handle))
        .collect()
}

fn session_records_for_scope(
    state: &State,
    scope: RegistrationScope,
) -> Vec<Arc<SessionInstallation>> {
    state
        .sessions
        .iter()
        .filter(|session| scope.applies_to(session.page, session.context))
        .cloned()
        .collect()
}

fn has_registration_for_page(state: &State, page: ObjectId, context: ObjectId) -> bool {
    state
        .registrations
        .iter()
        .any(|registration| registration.scope.applies_to(page, context))
}

fn retain_live_sessions(state: &mut State, session_ids: &[String]) {
    state.sessions.retain(|session| {
        let keep = session.initializing.load(Ordering::Acquire)
            || session_ids
                .iter()
                .any(|id| id == session.session_id.as_ref());
        if !keep {
            session.retired.store(true, Ordering::Release);
        }
        keep
    });
    let State {
        early, sessions, ..
    } = state;
    early.retain(|call| {
        sessions.iter().any(|session| {
            Arc::ptr_eq(session, &call.session) && !session.retired.load(Ordering::Acquire)
        })
    });
}

fn forget_session_from_state(state: &mut State, session: &Arc<SessionInstallation>) {
    session.retired.store(true, Ordering::Release);
    state
        .sessions
        .retain(|current| !Arc::ptr_eq(current, session));
    state
        .early
        .retain(|call| !Arc::ptr_eq(&call.session, session));
}

struct SessionInitializationGuard<'a> {
    state: &'a Mutex<State>,
    session: Arc<SessionInstallation>,
    armed: bool,
}

impl<'a> SessionInitializationGuard<'a> {
    fn new(state: &'a Mutex<State>, session: Arc<SessionInstallation>) -> Self {
        Self {
            state,
            session,
            armed: true,
        }
    }

    fn disarm(&mut self) {
        self.armed = false;
    }
}

impl Drop for SessionInitializationGuard<'_> {
    fn drop(&mut self) {
        if self.armed {
            self.session.retired.store(true, Ordering::Release);
            let mut state = self
                .state
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            forget_session_from_state(&mut state, &self.session);
        }
    }
}

fn call_is_pending(state: &State, id: u64) -> bool {
    state.pending.iter().any(|call| call.id == id)
}

fn retain_materialized_registrations(
    state: &mut State,
    pages: &[(ObjectId, ObjectId)],
    contexts: &[ObjectId],
) {
    state.registrations.retain(|binding| match binding.scope {
        #[cfg(any(feature = "context-bindings", test))]
        RegistrationScope::Context(context) => contexts.contains(&context),
        RegistrationScope::Page { page, context } => {
            contexts.contains(&context) && pages.contains(&(page, context))
        }
    });
}

pub(crate) struct Installation {
    connection: Arc<CdpConnection>,
    state: Mutex<State>,
    publication_changed: Notify,
    rejection_permits: Arc<Semaphore>,
    sender: mpsc::Sender<BindingCall>,
    receiver: tokio::sync::Mutex<mpsc::Receiver<BindingCall>>,
    _storage: ResourceLease,
}

#[derive(Deserialize)]
struct BrowserCall {
    schema: String,
    name: String,
    seq: u64,
    #[serde(default, rename = "serializedArgs")]
    serialized_args: Option<Value>,
}

struct ObservedBrowserCall<'a> {
    session_id: &'a str,
    execution_context_id: i64,
    payload: &'a str,
    call: BrowserCall,
}

fn observe_browser_call(
    event: &CdpEvent,
) -> Result<Option<ObservedBrowserCall<'_>>, Box<dyn Error + Send + Sync>> {
    if event.method != "Runtime.bindingCalled" {
        return Ok(None);
    }
    let binding_name = event
        .params
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| invalid("name"))?;
    if binding_name != BRIDGE_NAME {
        return Ok(None);
    }
    let session_id = event
        .session_id
        .as_deref()
        .ok_or_else(|| invalid("sessionId"))?;
    let execution_context_id = event
        .params
        .get("executionContextId")
        .and_then(Value::as_i64)
        .ok_or_else(|| invalid("executionContextId"))?;
    let payload = event
        .params
        .get("payload")
        .and_then(Value::as_str)
        .ok_or_else(|| invalid("payload"))?;
    if payload.len() > MAX_BINDING_PAYLOAD_BYTES {
        return Err(Box::new(HostError::ResourceExhausted(
            "binding payload exceeds byte limit".into(),
        )));
    }
    let call: BrowserCall = serde_json::from_str(payload)
        .map_err(|error| Box::new(error) as Box<dyn Error + Send + Sync>)?;
    if call.schema != BRIDGE_SCHEMA {
        return Err(Box::new(HostError::Backend(
            "Runtime binding event has an unsupported schema".into(),
        )));
    }
    Ok(Some(ObservedBrowserCall {
        session_id,
        execution_context_id,
        payload,
        call,
    }))
}

fn binding_route(
    topology: &CdpTopology,
    session_id: &str,
    execution_context_id: i64,
) -> Option<(ObjectId, ObjectId, ObjectId)> {
    let frame = topology.frame_execution_routes().find(|route| {
        route.session_id() == session_id && route.execution_context_id() == execution_context_id
    })?;
    let page = topology.page_object_id_for_frame(frame.object_id())?;
    let context = topology
        .page_session_routes()
        .find(|route| route.object_id() == page)?
        .browser_context_object_id();
    Some((context, page, frame.object_id()))
}

fn stage_early_call(
    state: &mut State,
    observed: &ObservedBrowserCall<'_>,
) -> Result<bool, HostError> {
    let Some(session) = state.sessions.iter().find(|session| {
        session.session_id.as_ref() == observed.session_id
            && !session.retired.load(Ordering::Acquire)
    }) else {
        return Ok(false);
    };
    let Some(registration) = state.registrations.iter().find(|registration| {
        registration.name.as_ref() == observed.call.name
            && registration.scope.applies_to(session.page, session.context)
    }) else {
        return Ok(false);
    };
    if state.early.len() == MAX_EARLY_CALLS {
        return Err(HostError::ResourceExhausted(format!(
            "early Page binding capacity exceeded ({MAX_EARLY_CALLS})"
        )));
    }
    state.early.push(EarlyCall {
        session: session.clone(),
        session_id: observed.session_id.to_owned().into_boxed_str(),
        execution_context_id: observed.execution_context_id,
        context: session.context,
        page: session.page,
        registration_id: registration.id,
        payload: observed.payload.to_owned().into_boxed_str(),
    });
    Ok(true)
}

fn take_ready_early_call(state: &mut State, topology: &CdpTopology) -> Option<EarlyCall> {
    let index = state.early.iter().position(|call| {
        binding_route(topology, &call.session_id, call.execution_context_id).is_some()
            || !state.sessions.iter().any(|session| {
                Arc::ptr_eq(session, &call.session) && !session.retired.load(Ordering::Acquire)
            })
            || !state.registrations.iter().any(|registration| {
                registration.id == call.registration_id
                    && registration.scope.applies_to(call.page, call.context)
            })
    })?;
    Some(state.early.remove(index))
}

fn discard_destroyed_world_from_state(state: &mut State, event: &CdpEvent) {
    let Some(session_id) = event.session_id.as_deref() else {
        return;
    };
    match event.method.as_str() {
        "Runtime.executionContextDestroyed" => {
            if let Some(id) = event
                .params
                .get("executionContextId")
                .and_then(Value::as_i64)
            {
                state.early.retain(|call| {
                    call.session_id.as_ref() != session_id || call.execution_context_id != id
                });
            }
        }
        "Runtime.executionContextsCleared" => {
            state
                .early
                .retain(|call| call.session_id.as_ref() != session_id);
        }
        _ => {}
    }
}

// A page may invoke the exposed CDP binding directly with malformed JSON or
// an oversized payload. Such input belongs to one call, not to the topology
// connection, so it must not become a terminal driver error.
fn observe_browser_call_nonfatal(
    event: &CdpEvent,
) -> Result<Option<ObservedBrowserCall<'_>>, Box<dyn Error + Send + Sync>> {
    match observe_browser_call(event) {
        Err(_)
            if event.method == "Runtime.bindingCalled"
                && event.params.get("name").and_then(Value::as_str) == Some(BRIDGE_NAME) =>
        {
            Ok(None)
        }
        result => result,
    }
}

fn is_per_call_rejection(error: &HostError) -> bool {
    matches!(
        error,
        HostError::InvalidArgument(_) | HostError::ResourceExhausted(_)
    )
}

fn rejection_source(name: &str, seq: u64, error: &str) -> Result<String, HostError> {
    let name = serde_json::to_string(name).map_err(HostError::backend)?;
    let error = serde_json::to_string(error).map_err(HostError::backend)?;
    Ok(format!(
        "globalThis.{CONTROLLER_NAME}?.deliver({{name:{name},seq:{seq},error:new Error({error})}})"
    ))
}

pub(crate) const fn requirements() -> ResourceRequirements {
    // Includes fixed queue cells and the maximum retained payload bodies. The
    // mpsc allocation is bounded by the same item count and never grows. Each
    // pending call owns a name separately from its queued/foreign descriptor.
    let bytes = (std::mem::size_of::<Installation>()
        + std::mem::size_of::<State>()
        + MAX_BINDINGS * (std::mem::size_of::<Registration>() + MAX_BINDING_NAME_BYTES)
        + MAX_PENDING_CALLS
            * (std::mem::size_of::<PendingCall>()
                + std::mem::size_of::<BindingCall>()
                + 2 * MAX_BINDING_NAME_BYTES
                + MAX_ARGUMENT_BYTES)
        + MAX_EARLY_CALLS
            * (std::mem::size_of::<EarlyCall>() + MAX_SESSION_ID_BYTES + MAX_BINDING_PAYLOAD_BYTES)
        + MAX_REJECTION_TASKS * MAX_REJECTION_TASK_BYTES
        + MAX_BINDING_SESSIONS
            * (std::mem::size_of::<Arc<SessionInstallation>>()
                + std::mem::size_of::<SessionInstallation>()
                + MAX_SESSION_ID_BYTES
                + MAX_BINDINGS
                    * (std::mem::size_of::<InstalledScript>() + MAX_SCRIPT_IDENTIFIER_BYTES)))
        as u64;
    ResourceRequirements::none()
        .with_class(ResourceClass::BackendStateBytes, bytes, bytes)
        .with_metadata(1, 1)
}

impl Installation {
    fn has_publication_sources(&self) -> Result<bool, HostError> {
        let state = self.state.lock().map_err(HostError::backend)?;
        Ok(has_publication_sources(&state))
    }

    pub(crate) fn new(
        connection: Arc<CdpConnection>,
        resources: &ProductionResourceLedger,
    ) -> Result<Arc<Self>, HostError> {
        let plan = requirements();
        let ledger = resources.claim(plan).map_err(HostError::backend)?;
        let storage = ledger
            .reserve(
                SCOPE,
                ResourceClass::BackendStateBytes,
                plan.class(ResourceClass::BackendStateBytes).global,
            )
            .map_err(HostError::backend)?;
        let mut registrations = Vec::new();
        registrations
            .try_reserve_exact(MAX_BINDINGS)
            .map_err(|error| HostError::ResourceExhausted(error.to_string()))?;
        let mut pending = Vec::new();
        pending
            .try_reserve_exact(MAX_PENDING_CALLS)
            .map_err(|error| HostError::ResourceExhausted(error.to_string()))?;
        let mut early = Vec::new();
        early
            .try_reserve_exact(MAX_EARLY_CALLS)
            .map_err(|error| HostError::ResourceExhausted(error.to_string()))?;
        let mut sessions = Vec::new();
        sessions
            .try_reserve_exact(MAX_BINDING_SESSIONS)
            .map_err(|error| HostError::ResourceExhausted(error.to_string()))?;
        let (sender, receiver) = mpsc::channel(MAX_PENDING_CALLS);
        Ok(Arc::new(Self {
            connection,
            state: Mutex::new(State {
                next_call: 1,
                next_registration: 1,
                registrations,
                pending,
                early,
                sessions,
            }),
            publication_changed: Notify::new(),
            rejection_permits: Arc::new(Semaphore::new(MAX_REJECTION_TASKS)),
            sender,
            receiver: tokio::sync::Mutex::new(receiver),
            _storage: storage,
        }))
    }

    fn binding_source(name: &str, needs_handle: bool) -> Result<String, HostError> {
        let encoded = serde_json::to_string(name).map_err(HostError::backend)?;
        Ok(format!(
            "globalThis.{CONTROLLER_NAME}?.add({encoded}, {needs_handle});"
        ))
    }

    async fn install_base(&self, session_id: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
        let binding = self.connection.send_command_with_session(
            "Runtime.addBinding",
            serde_json::json!({ "name": BRIDGE_NAME }),
            Some(session_id),
        );
        let script = self.connection.send_command_with_session(
            "Page.addScriptToEvaluateOnNewDocument",
            serde_json::json!({
                "source": CONTROLLER_SOURCE,
                "runImmediately": true,
            }),
            Some(session_id),
        );
        tokio::try_join!(binding, script)?;
        Ok(())
    }

    async fn install_name(
        &self,
        session_id: &str,
        name: &str,
        needs_handle: bool,
    ) -> Result<String, Box<dyn Error + Send + Sync>> {
        let value = self
            .connection
            .send_command_with_session(
                "Page.addScriptToEvaluateOnNewDocument",
                serde_json::json!({
                    "source": Self::binding_source(name, needs_handle)?,
                    "runImmediately": true,
                }),
                Some(session_id),
            )
            .await?;
        value
            .get("identifier")
            .and_then(Value::as_str)
            .map(str::to_owned)
            .ok_or_else(|| {
                Box::new(HostError::Backend(
                    "Page.addScriptToEvaluateOnNewDocument is missing identifier".into(),
                )) as Box<dyn Error + Send + Sync>
            })
    }

    fn add_registration(
        &self,
        scope: RegistrationScope,
        name: &str,
        needs_handle: bool,
    ) -> Result<(Registration, Vec<Arc<SessionInstallation>>), HostError> {
        let mut state = self.state.lock().map_err(HostError::backend)?;
        let id = add_registration_to_state(&mut state, scope, name, needs_handle)?;
        let registration = state
            .registrations
            .iter()
            .find(|registration| registration.id == id)
            .expect("just inserted registration")
            .clone();
        let sessions = session_records_for_scope(&state, scope);
        Ok((registration, sessions))
    }

    async fn begin_session(
        &self,
        session_id: &str,
        page: ObjectId,
        context: ObjectId,
    ) -> Result<
        (
            Arc<SessionInstallation>,
            tokio::sync::OwnedMutexGuard<Vec<InstalledScript>>,
        ),
        HostError,
    > {
        if session_id.is_empty() || session_id.len() > MAX_SESSION_ID_BYTES {
            return Err(HostError::ResourceExhausted(
                "binding session ID exceeds byte limit".into(),
            ));
        }
        let mut scripts = Vec::new();
        scripts
            .try_reserve_exact(MAX_BINDINGS)
            .map_err(|error| HostError::ResourceExhausted(error.to_string()))?;
        let session = Arc::new(SessionInstallation {
            session_id: session_id.to_owned().into_boxed_str(),
            page,
            context,
            initializing: AtomicBool::new(true),
            retired: AtomicBool::new(false),
            scripts: Arc::new(tokio::sync::Mutex::new(scripts)),
        });
        // Publish only after the installer owns this session gate. A concurrent
        // registration that sees the pre-publication record then waits for the
        // controller and its initial scripts, never installing ahead of them.
        let scripts = session.scripts.clone().lock_owned().await;
        let mut state = self.state.lock().map_err(HostError::backend)?;
        if state.sessions.len() == MAX_BINDING_SESSIONS {
            return Err(HostError::ResourceExhausted(format!(
                "Page binding session capacity exceeded ({MAX_BINDING_SESSIONS})"
            )));
        }
        if state
            .sessions
            .iter()
            .any(|current| current.session_id == session.session_id)
        {
            return Err(HostError::Backend(
                "Page binding session initialized twice".into(),
            ));
        }
        state.sessions.push(session.clone());
        Ok((session, scripts))
    }

    fn forget_session(&self, session: &Arc<SessionInstallation>) -> Result<(), HostError> {
        session.retired.store(true, Ordering::Release);
        let mut state = self.state.lock().map_err(HostError::backend)?;
        forget_session_from_state(&mut state, session);
        Ok(())
    }

    fn registrations_for_session(
        &self,
        session: &SessionInstallation,
    ) -> Result<Vec<Registration>, HostError> {
        let state = self.state.lock().map_err(HostError::backend)?;
        Ok(state
            .registrations
            .iter()
            .filter(|binding| binding.scope.applies_to(session.page, session.context))
            .cloned()
            .collect())
    }

    fn registration_is_live(&self, id: u64) -> Result<bool, HostError> {
        Ok(self
            .state
            .lock()
            .map_err(HostError::backend)?
            .registrations
            .iter()
            .any(|binding| binding.id == id))
    }

    async fn install_session_registration(
        &self,
        session: &SessionInstallation,
        registration: &Registration,
        scripts: &mut Vec<InstalledScript>,
    ) -> Result<(), HostError> {
        if session.retired.load(Ordering::Acquire)
            || scripts
                .iter()
                .any(|script| script.registration_id == registration.id)
            || !self.registration_is_live(registration.id)?
        {
            return Ok(());
        }
        let identifier = self
            .install_name(
                &session.session_id,
                &registration.name,
                registration.needs_handle,
            )
            .await
            .map_err(HostError::backend)?;
        if identifier.len() > MAX_SCRIPT_IDENTIFIER_BYTES {
            let _ = self
                .connection
                .send_command_with_session(
                    "Page.removeScriptToEvaluateOnNewDocument",
                    serde_json::json!({ "identifier": identifier }),
                    Some(&session.session_id),
                )
                .await;
            return Err(HostError::ResourceExhausted(
                "binding script identifier exceeds byte limit".into(),
            ));
        }
        scripts.push(InstalledScript {
            registration_id: registration.id,
            identifier: identifier.into_boxed_str(),
        });
        Ok(())
    }

    async fn ensure_session_registration(
        &self,
        session: &SessionInstallation,
        registration: &Registration,
    ) -> Result<(), HostError> {
        let mut scripts = session.scripts.lock().await;
        self.install_session_registration(session, registration, &mut scripts)
            .await
    }

    async fn rollback_registration(&self, registration: &Registration) -> Result<(), HostError> {
        // This removes Rust routing and future-document scripts. CDP
        // runImmediately may already have installed globalThis[name] in a
        // current document, so this is not an atomic browser-side rollback:
        // a stale function there may remain until navigation or Page close.
        let sessions = {
            let mut state = self.state.lock().map_err(HostError::backend)?;
            state
                .registrations
                .retain(|item| item.id != registration.id);
            state
                .early
                .retain(|call| call.registration_id != registration.id);
            state.sessions.clone()
        };
        for session in sessions {
            let mut scripts = session.scripts.lock().await;
            if let Some(index) = scripts
                .iter()
                .position(|script| script.registration_id == registration.id)
            {
                let script = scripts.swap_remove(index);
                let _ = self
                    .connection
                    .send_command_with_session(
                        "Page.removeScriptToEvaluateOnNewDocument",
                        serde_json::json!({ "identifier": script.identifier }),
                        Some(&session.session_id),
                    )
                    .await;
            }
        }
        Ok(())
    }

    fn remove_context(&self, context: ObjectId) -> Result<(), HostError> {
        let mut state = self.state.lock().map_err(HostError::backend)?;
        remove_context_from_state(&mut state, context);
        drop(state);
        self.publication_changed.notify_waiters();
        Ok(())
    }

    fn remove_page(&self, page: ObjectId) -> Result<(), HostError> {
        let mut state = self.state.lock().map_err(HostError::backend)?;
        remove_page_from_state(&mut state, page);
        drop(state);
        self.publication_changed.notify_waiters();
        Ok(())
    }

    fn reconcile(
        &self,
        pages: &[(ObjectId, ObjectId)],
        contexts: &[ObjectId],
        session_ids: &[String],
    ) -> Result<(), HostError> {
        let mut state = self.state.lock().map_err(HostError::backend)?;
        retain_materialized_registrations(&mut state, pages, contexts);
        state.pending.retain(|call| {
            contexts.contains(&call.context) && pages.contains(&(call.page, call.context))
        });
        retain_live_sessions(&mut state, session_ids);
        drop(state);
        self.publication_changed.notify_waiters();
        Ok(())
    }

    fn binding_mode(
        &self,
        page: ObjectId,
        context: ObjectId,
        name: &str,
        expected_registration_id: Option<u64>,
    ) -> Result<Option<(u64, bool)>, HostError> {
        Ok(self
            .state
            .lock()
            .map_err(HostError::backend)?
            .registrations
            .iter()
            .find(|binding| {
                binding.scope.applies_to(page, context)
                    && binding.name.as_ref() == name
                    && expected_registration_id.is_none_or(|id| binding.id == id)
            })
            .map(|binding| (binding.id, binding.needs_handle)))
    }

    fn has_registration_for_page(
        &self,
        page: ObjectId,
        context: ObjectId,
    ) -> Result<bool, HostError> {
        let state = self.state.lock().map_err(HostError::backend)?;
        Ok(has_registration_for_page(&state, page, context))
    }

    fn admit_call(
        &self,
        context: ObjectId,
        page: ObjectId,
        frame: ObjectId,
        call: &BrowserCall,
        expected_registration_id: Option<u64>,
        expected_session: Option<&Arc<SessionInstallation>>,
    ) -> Result<(), HostError> {
        let Some((registration_id, needs_handle)) =
            self.binding_mode(page, context, &call.name, expected_registration_id)?
        else {
            return Ok(());
        };
        if call.name.len() > MAX_BINDING_NAME_BYTES {
            return Err(HostError::ResourceExhausted(
                "binding call name exceeds byte limit".into(),
            ));
        }
        if call.seq > 9_007_199_254_740_991 {
            return Err(HostError::ResourceExhausted(
                "binding call sequence exceeds JavaScript safe integer range".into(),
            ));
        }
        let arguments_json = if needs_handle {
            "[]".to_owned()
        } else {
            serde_json::to_string(call.serialized_args.as_ref().ok_or_else(|| {
                HostError::InvalidArgument("value binding call is missing serializedArgs".into())
            })?)
            .map_err(HostError::backend)?
        };
        if arguments_json.len() > MAX_ARGUMENT_BYTES {
            return Err(HostError::ResourceExhausted(
                "binding call arguments exceed byte limit".into(),
            ));
        }
        let mut state = self.state.lock().map_err(HostError::backend)?;
        if !state.registrations.iter().any(|registration| {
            registration.id == registration_id && registration.scope.applies_to(page, context)
        }) {
            return Ok(());
        }
        if let Some(expected_session) = expected_session
            && !state.sessions.iter().any(|session| {
                Arc::ptr_eq(session, expected_session) && !session.retired.load(Ordering::Acquire)
            })
        {
            return Ok(());
        }
        if state.pending.len() == MAX_PENDING_CALLS {
            return Err(HostError::ResourceExhausted(format!(
                "pending Page binding capacity exceeded ({MAX_PENDING_CALLS})"
            )));
        }
        let id = state.next_call;
        state.next_call = id.checked_add(1).ok_or_else(|| {
            HostError::ResourceExhausted("binding call ID space exhausted".into())
        })?;
        state.pending.push(PendingCall {
            id,
            context,
            page,
            frame,
            name: call.name.clone().into_boxed_str(),
            sequence: call.seq,
            needs_handle,
            handle_taken: false,
            published: false,
            delivering: false,
        });
        let descriptor = BindingCall {
            id,
            context,
            page,
            frame,
            name: call.name.clone(),
            arguments_json,
            needs_handle,
        };
        if let Err(error) = self.sender.try_send(descriptor) {
            state.pending.retain(|pending| pending.id != id);
            return Err(HostError::ResourceExhausted(format!(
                "Page binding delivery queue is unavailable: {error}"
            )));
        }
        Ok(())
    }

    fn stage_call(&self, observed: &ObservedBrowserCall<'_>) -> Result<(), HostError> {
        let mut state = self.state.lock().map_err(HostError::backend)?;
        stage_early_call(&mut state, observed).map(|_| ())
    }

    fn discard_destroyed_world(&self, event: &CdpEvent) -> Result<(), HostError> {
        if !matches!(
            event.method.as_str(),
            "Runtime.executionContextDestroyed" | "Runtime.executionContextsCleared"
        ) {
            return Ok(());
        }
        let mut state = self.state.lock().map_err(HostError::backend)?;
        discard_destroyed_world_from_state(&mut state, event);
        Ok(())
    }

    fn drain_early_calls(&self, topology: &CdpTopology) -> Result<(), HostError> {
        loop {
            let early = {
                let mut state = self.state.lock().map_err(HostError::backend)?;
                take_ready_early_call(&mut state, topology)
            };
            let Some(early) = early else {
                return Ok(());
            };
            let Some((context, page, frame)) =
                binding_route(topology, &early.session_id, early.execution_context_id)
            else {
                continue;
            };
            if context != early.context || page != early.page {
                continue;
            }
            let Ok(call) = serde_json::from_str::<BrowserCall>(&early.payload) else {
                continue;
            };
            match self.admit_call(
                context,
                page,
                frame,
                &call,
                Some(early.registration_id),
                Some(&early.session),
            ) {
                Ok(()) => {}
                Err(error) if is_per_call_rejection(&error) => {
                    self.reject_call(&early.session_id, early.execution_context_id, &call, &error);
                }
                Err(error) => return Err(error),
            }
        }
    }

    fn reject_call(
        &self,
        session_id: &str,
        execution_context_id: i64,
        call: &BrowserCall,
        error: &HostError,
    ) {
        if session_id.len() > MAX_SESSION_ID_BYTES
            || call.name.len() > MAX_BINDING_NAME_BYTES
            || call.seq > 9_007_199_254_740_991
        {
            return;
        }
        let Ok(permit) = self.rejection_permits.clone().try_acquire_owned() else {
            return;
        };
        let Ok(runtime) = tokio::runtime::Handle::try_current() else {
            return;
        };
        let Ok(expression) = rejection_source(&call.name, call.seq, &error.to_string()) else {
            return;
        };
        let connection = self.connection.clone();
        let session_id = session_id.to_owned();
        runtime.spawn(async move {
            let _permit = permit;
            let _ = connection
                .send_command_with_session(
                    "Runtime.evaluate",
                    serde_json::json!({
                        "expression": expression,
                        "contextId": execution_context_id,
                        "returnByValue": true,
                    }),
                    Some(&session_id),
                )
                .await;
        });
    }

    async fn next_call(&self, stopped: &CancellationToken) -> Option<BindingCall> {
        let mut receiver = self.receiver.lock().await;
        loop {
            let descriptor = tokio::select! {
                biased;
                () = stopped.cancelled() => return None,
                value = receiver.recv() => value?,
            };
            let current = {
                let state = self.state.lock().ok()?;
                call_is_pending(&state, descriptor.id)
            };
            if current {
                return Some(descriptor);
            }
        }
    }

    fn acknowledge_publication(&self, id: u64) -> Result<(), HostError> {
        let mut state = self.state.lock().map_err(HostError::backend)?;
        let call = state
            .pending
            .iter_mut()
            .find(|call| call.id == id)
            .ok_or_else(|| HostError::InvalidArgument("unknown binding call ID".into()))?;
        call.published = true;
        drop(state);
        self.publication_changed.notify_waiters();
        Ok(())
    }

    fn publication_call(&self, id: u64) -> Result<PendingOperation, HostError> {
        let state = self.state.lock().map_err(HostError::backend)?;
        let call = state
            .pending
            .iter()
            .find(|call| call.id == id)
            .ok_or_else(|| HostError::InvalidArgument("unknown binding call ID".into()))?;
        Ok(PendingOperation {
            frame: call.frame,
            name: call.name.clone(),
            sequence: call.sequence,
        })
    }

    async fn wait_publications(&self, stopped: &CancellationToken) -> Result<(), HostError> {
        let watermark = self
            .state
            .lock()
            .map_err(HostError::backend)?
            .next_call
            .saturating_sub(1);
        wait_for_publications(&self.state, &self.publication_changed, watermark, stopped).await
    }

    fn begin_take(&self, id: u64) -> Result<PendingOperation, HostError> {
        let mut state = self.state.lock().map_err(HostError::backend)?;
        let call = state
            .pending
            .iter_mut()
            .find(|call| call.id == id)
            .ok_or_else(|| HostError::InvalidArgument("unknown binding call ID".into()))?;
        if !call.needs_handle {
            return Err(HostError::InvalidArgument(
                "value binding call has no Handle argument".into(),
            ));
        }
        if call.handle_taken {
            return Err(HostError::InvalidArgument(
                "binding Handle argument has already been taken".into(),
            ));
        }
        call.handle_taken = true;
        Ok(PendingOperation {
            frame: call.frame,
            name: call.name.clone(),
            sequence: call.sequence,
        })
    }

    fn begin_delivery(self: &Arc<Self>, id: u64) -> Result<Delivery, HostError> {
        let call = {
            let mut state = self.state.lock().map_err(HostError::backend)?;
            let call = state
                .pending
                .iter_mut()
                .find(|call| call.id == id)
                .ok_or_else(|| HostError::InvalidArgument("unknown binding call ID".into()))?;
            if call.delivering {
                return Err(HostError::InvalidArgument(
                    "binding result delivery is already in progress".into(),
                ));
            }
            call.delivering = true;
            PendingOperation {
                frame: call.frame,
                name: call.name.clone(),
                sequence: call.sequence,
            }
        };
        Ok(Delivery {
            installation: self.clone(),
            id,
            call,
        })
    }

    fn finish_delivery(&self, id: u64) {
        if let Ok(mut state) = self.state.lock() {
            state.pending.retain(|call| call.id != id);
            drop(state);
            self.publication_changed.notify_waiters();
        }
    }
}

async fn wait_for_publications(
    state: &Mutex<State>,
    changed: &Notify,
    watermark: u64,
    stopped: &CancellationToken,
) -> Result<(), HostError> {
    loop {
        let notified = changed.notified();
        tokio::pin!(notified);
        notified.as_mut().enable();
        let complete = state
            .lock()
            .map_err(HostError::backend)?
            .pending
            .iter()
            .all(|call| call.id > watermark || call.published);
        if complete {
            return Ok(());
        }
        tokio::select! {
            biased;
            () = stopped.cancelled() => return Err(HostError::TargetClosed),
            () = &mut notified => {}
        }
    }
}

struct Delivery {
    installation: Arc<Installation>,
    id: u64,
    call: PendingOperation,
}

impl Drop for Delivery {
    fn drop(&mut self) {
        self.installation.finish_delivery(self.id);
    }
}

pub(crate) struct Initializer<Downstream> {
    downstream: Downstream,
    installation: Arc<Installation>,
}

impl<Downstream> Initializer<Downstream> {
    pub(crate) fn new(downstream: Downstream, installation: Arc<Installation>) -> Self {
        Self {
            downstream,
            installation,
        }
    }
}

impl<Downstream> ChromiumSessionInitializer for Initializer<Downstream>
where
    Downstream: ChromiumSessionInitializer,
{
    fn initialize<'a>(
        &'a self,
        session_id: &'a str,
        target_id: &'a str,
        ownership: pwright_backend_cdp_topology::CdpContextOwnership,
        is_page: bool,
    ) -> ChromiumSessionInitializationFuture<'a> {
        Box::pin(async move {
            let downstream = self
                .downstream
                .initialize(session_id, target_id, ownership, is_page);
            let base = self.installation.install_base(session_id);
            tokio::try_join!(downstream, base).map(|_| ())
        })
    }

    fn initialize_scoped<'a>(
        &'a self,
        target: ChromiumSessionInitializationTarget<'a>,
    ) -> ChromiumSessionInitializationFuture<'a> {
        Box::pin(async move {
            let (session, mut scripts) = self
                .installation
                .begin_session(target.session_id, target.page, target.context)
                .await?;
            let mut cancellation =
                SessionInitializationGuard::new(&self.installation.state, session.clone());
            let outcome: Result<(), Box<dyn Error + Send + Sync>> = async {
                let downstream = self.downstream.initialize_scoped(target);
                let base = self.installation.install_base(target.session_id);
                tokio::try_join!(downstream, base)?;
                for registration in self.installation.registrations_for_session(&session)? {
                    self.installation
                        .install_session_registration(&session, &registration, &mut scripts)
                        .await?;
                }
                Ok(())
            }
            .await;
            session.initializing.store(false, Ordering::Release);
            if outcome.is_err() {
                self.installation.forget_session(&session)?;
            }
            cancellation.disarm();
            outcome
        })
    }
}

pub(crate) struct Observer {
    installation: Arc<Installation>,
    downstream: Arc<dyn ChromiumFrameEventObserver>,
}

impl Observer {
    pub(crate) fn new(
        installation: Arc<Installation>,
        downstream: Arc<dyn ChromiumFrameEventObserver>,
    ) -> Self {
        Self {
            installation,
            downstream,
        }
    }
}

impl ChromiumFrameEventObserver for Observer {
    fn observes_page(&self, page: ObjectId) -> bool {
        self.downstream.observes_page(page)
    }

    fn prepare_page(&self, topology: &CdpTopology, page: ObjectId) -> FrameObservationResult {
        self.downstream.prepare_page(topology, page)
    }

    fn discard_prepared_page(
        &self,
        topology: &CdpTopology,
        page: ObjectId,
    ) -> FrameObservationResult {
        self.downstream.discard_prepared_page(topology, page)?;
        if topology
            .materialized_page_memberships()
            .any(|(member, _context)| member == page)
        {
            return Ok(());
        }
        self.installation
            .remove_page(page)
            .map_err(|error| Box::new(error) as Box<dyn Error + Send + Sync>)
    }

    fn initialize_frame(
        &self,
        topology: &CdpTopology,
        session: &str,
        frame: &page::Frame,
    ) -> FrameObservationResult {
        self.downstream.initialize_frame(topology, session, frame)
    }

    fn reconcile(&self, topology: &CdpTopology) -> FrameObservationResult {
        self.downstream.reconcile(topology)?;
        let pages = topology.materialized_page_memberships().collect::<Vec<_>>();
        let contexts = topology
            .browser_context_routes()
            .map(|route| route.object_id())
            .collect::<Vec<_>>();
        let sessions = topology
            .target_session_routes()
            .map(|route| route.session_id().to_owned())
            .collect::<Vec<_>>();
        self.installation
            .reconcile(&pages, &contexts, &sessions)
            .and_then(|()| self.installation.drain_early_calls(topology))
            .map_err(|error| Box::new(error) as Box<dyn Error + Send + Sync>)
    }

    fn event(&self, topology: &CdpTopology, event: &CdpEvent) -> FrameObservationResult {
        self.downstream.event(topology, event)?;
        self.installation
            .discard_destroyed_world(event)
            .map_err(|error| Box::new(error) as Box<dyn Error + Send + Sync>)?;
        // A route may become ready on this event. Preserve CDP call order by
        // replaying older staged calls before admitting a later binding call.
        self.installation
            .drain_early_calls(topology)
            .map_err(|error| Box::new(error) as Box<dyn Error + Send + Sync>)?;
        if let Some(observed) = observe_browser_call_nonfatal(event)? {
            let outcome = if let Some((context, page, frame)) =
                binding_route(topology, observed.session_id, observed.execution_context_id)
            {
                self.installation
                    .admit_call(context, page, frame, &observed.call, None, None)
            } else {
                self.installation.stage_call(&observed)
            };
            match outcome {
                Ok(()) => {}
                Err(error) if is_per_call_rejection(&error) => {
                    // The event pump must return before a CDP command can
                    // receive its response. Reject only this browser Promise.
                    self.installation.reject_call(
                        observed.session_id,
                        observed.execution_context_id,
                        &observed.call,
                        &error,
                    );
                }
                Err(error) => return Err(Box::new(error)),
            }
        }
        Ok(())
    }

    fn seal(&self) {
        if let Ok(mut state) = self.installation.state.lock() {
            state.early.clear();
        }
        self.downstream.seal();
    }
}

fn invalid(field: &str) -> Box<dyn Error + Send + Sync> {
    Box::new(HostError::Backend(format!(
        "Runtime binding event is missing {field}"
    )))
}

impl HostSession {
    pub(crate) fn page_binding_frame_for_reload(
        &self,
        page_id: &str,
    ) -> Result<Option<String>, HostError> {
        let page = page_id
            .parse::<u64>()
            .ok()
            .filter(|id| *id != 0)
            .map(ObjectId::new)
            .ok_or_else(|| HostError::InvalidArgument("invalid portable Page ID".into()))?;
        let (context, installation) = {
            let state = self.state.lock().map_err(HostError::backend)?;
            let live = state.as_ref().ok_or(HostError::TargetClosed)?;
            let topology = live.composition.topology().map_err(HostError::backend)?;
            let context = topology
                .topology()
                .materialized_page_memberships()
                .find_map(|(member, context)| (member == page).then_some(context))
                .ok_or(HostError::TargetClosed)?;
            (context, live.page_bindings.clone())
        };
        if !installation.has_registration_for_page(page, context)? {
            return Ok(None);
        }
        self.require_visible_page_id(page)?;
        let frame = self.frame_trees.get(page)?.main_frame_snapshot()?.id();
        Ok(Some(frame.get().to_string()))
    }

    pub(crate) async fn drain_frame_binding_publications(
        &self,
        frame_id: &str,
    ) -> Result<(), HostError> {
        // The current document's controller resolves this Promise only after
        // its binding callbacks have started and been acknowledged. It never
        // waits for calls in another Page or a prior document. A document
        // without an installed controller has no such publications to drain.
        self.frame_evaluate(
            frame_id,
            EvaluateParams::expression(format!(
                "globalThis.{CONTROLLER_NAME}?.drainPublications?.()"
            )),
        )
        .await
        .map(|_| ())
    }

    async fn register_binding(
        &self,
        scope: RegistrationScope,
        sessions: Vec<String>,
        name: String,
        needs_handle: bool,
    ) -> Result<(), HostError> {
        let installation = self.page_binding_installation()?;
        // The initial Page/Context lookup in the public entrypoint happens
        // before this call. Recheck liveness and reserve the name while
        // holding the same host-state lock used by close retirement. A
        // page-free Context may otherwise close between that lookup and the
        // separate binding registry insertion, leaving a ghost name.
        let (registration, records) = {
            let state = self.state.lock().map_err(HostError::backend)?;
            let live = state.as_ref().ok_or(HostError::TargetClosed)?;
            let topology = live.composition.topology().map_err(HostError::backend)?;
            if !scope.is_live(topology.topology()) {
                return Err(HostError::TargetClosed);
            }
            installation.add_registration(scope, &name, needs_handle)?
        };
        let mut installed = Vec::new();
        let mut failure = None;
        for record in &records {
            if let Err(error) = installation
                .ensure_session_registration(record, &registration)
                .await
            {
                failure = Some(error);
                break;
            }
        }
        if failure.is_none() {
            for session in &sessions {
                if records
                    .iter()
                    .any(|record| record.session_id.as_ref() == session)
                {
                    continue;
                }
                match installation
                    .install_name(session, &name, needs_handle)
                    .await
                {
                    Ok(identifier) => installed.push((session.clone(), identifier)),
                    Err(error) => {
                        failure = Some(HostError::backend(error));
                        break;
                    }
                }
            }
        }
        if let Some(error) = failure {
            installation.rollback_registration(&registration).await?;
            for (session, identifier) in installed {
                let _ = installation
                    .connection
                    .send_command_with_session(
                        "Page.removeScriptToEvaluateOnNewDocument",
                        serde_json::json!({ "identifier": identifier }),
                        Some(&session),
                    )
                    .await;
            }
            return Err(error);
        }
        Ok(())
    }

    pub async fn page_expose_binding(
        &self,
        page_id: &str,
        name: String,
        needs_handle: bool,
    ) -> Result<(), HostError> {
        let page = page_id
            .parse::<u64>()
            .ok()
            .filter(|id| *id != 0)
            .map(ObjectId::new)
            .ok_or_else(|| HostError::InvalidArgument("invalid portable Page ID".into()))?;
        self.require_visible_page_id(page)?;
        let (context, sessions) = {
            let state = self.state.lock().map_err(HostError::backend)?;
            let live = state.as_ref().ok_or(HostError::TargetClosed)?;
            let topology = live.composition.topology().map_err(HostError::backend)?;
            let topology = topology.topology();
            let (target, context) = topology
                .page_session_routes()
                .find(|route| route.object_id() == page)
                .map(|route| {
                    (
                        route.target_id().to_owned(),
                        route.browser_context_object_id(),
                    )
                })
                .ok_or(HostError::TargetClosed)?;
            let sessions = topology
                .target_session_routes()
                .filter(|route| route.page_target_id() == target)
                .map(|route| route.session_id().to_owned())
                .collect::<Vec<_>>();
            (context, sessions)
        };
        self.register_binding(
            RegistrationScope::Page { page, context },
            sessions,
            name,
            needs_handle,
        )
        .await
    }

    #[cfg(feature = "context-bindings")]
    pub async fn context_expose_binding(
        &self,
        context_id: &str,
        name: String,
        needs_handle: bool,
    ) -> Result<(), HostError> {
        let context = context_id
            .parse::<u64>()
            .ok()
            .filter(|id| *id != 0)
            .map(ObjectId::new)
            .ok_or_else(|| {
                HostError::InvalidArgument("invalid portable BrowserContext ID".into())
            })?;
        self.require_visible_context_id(context)?;
        let sessions = {
            let state = self.state.lock().map_err(HostError::backend)?;
            let live = state.as_ref().ok_or(HostError::TargetClosed)?;
            let topology = live.composition.topology().map_err(HostError::backend)?;
            let topology = topology.topology();
            if !topology
                .browser_context_routes()
                .any(|route| route.object_id() == context)
            {
                return Err(HostError::TargetClosed);
            }
            let page_targets = topology
                .page_session_routes()
                .filter(|route| route.browser_context_object_id() == context)
                .map(|route| route.target_id().to_owned())
                .collect::<Vec<_>>();
            topology
                .target_session_routes()
                .filter(|route| {
                    page_targets
                        .iter()
                        .any(|target| target == route.page_target_id())
                })
                .map(|route| route.session_id().to_owned())
                .collect::<Vec<_>>()
        };
        self.register_binding(
            RegistrationScope::Context(context),
            sessions,
            name,
            needs_handle,
        )
        .await
    }

    pub(crate) fn retire_page_binding_registrations(
        &self,
        page: ObjectId,
    ) -> Result<(), HostError> {
        let state = self.state.lock().map_err(HostError::backend)?;
        if let Some(live) = state.as_ref() {
            live.page_bindings.remove_page(page)?;
        }
        Ok(())
    }

    pub(crate) fn retire_context_binding_registrations(
        &self,
        context: ObjectId,
    ) -> Result<(), HostError> {
        let state = self.state.lock().map_err(HostError::backend)?;
        if let Some(live) = state.as_ref() {
            live.page_bindings.remove_context(context)?;
        }
        Ok(())
    }

    pub async fn next_page_binding_call(&self) -> Option<BindingCall> {
        let installation = {
            let state = self.state.lock().ok()?;
            state.as_ref()?.page_bindings.clone()
        };
        installation.next_call(&self.stop).await
    }

    pub async fn acknowledge_page_binding_call(&self, call_id: u64) -> Result<(), HostError> {
        let installation = self.page_binding_installation()?;
        let call = installation.publication_call(call_id)?;
        self.frame_evaluate(
            &call.frame.get().to_string(),
            EvaluateParams::expression(publication_source(&call)?),
        )
        .await?;
        installation.acknowledge_publication(call_id)
    }

    pub(crate) async fn flush_page_binding_publications(
        &self,
        frame_id: &str,
    ) -> Result<(), HostError> {
        let installation = self.page_binding_installation()?;
        // Without any registered or in-flight binding, this action cannot
        // have a callback publication to drain. In particular, a DOM action
        // may already have navigated away from this Frame's old world.
        if !installation.has_publication_sources()? {
            return Ok(());
        }
        // Calls created synchronously by the DOM action carry a controller
        // Promise that resolves only after the foreign runtime starts the
        // callback and explicitly acknowledges its call ID. This same-frame
        // handshake uses no timer and does not await the callback's result.
        self.frame_evaluate(
            frame_id,
            EvaluateParams::expression(format!(
                "globalThis.{CONTROLLER_NAME}?.drainPublications?.()"
            )),
        )
        .await?;
        installation.wait_publications(&self.stop).await
    }

    fn page_binding_installation(&self) -> Result<Arc<Installation>, HostError> {
        let state = self.state.lock().map_err(HostError::backend)?;
        Ok(state
            .as_ref()
            .ok_or(HostError::TargetClosed)?
            .page_bindings
            .clone())
    }

    pub async fn take_page_binding_handle(&self, call_id: u64) -> Result<PendingHandle, HostError> {
        let installation = self.page_binding_installation()?;
        let call = installation.begin_take(call_id)?;
        let sequence = EvaluationValue::number(call.sequence as f64)
            .map_err(|_| HostError::InvalidArgument("invalid binding call sequence".into()))?;
        let argument = EvaluationValue::Object {
            id: None,
            properties: vec![
                EvaluationProperty::new("name", EvaluationValue::String(call.name.into_string())),
                EvaluationProperty::new("seq", sequence),
            ],
        };
        self.frame_prepare_handle(
            &call.frame.get().to_string(),
            EvaluateHandleParams::function(
                format!(
                    "arg => {{ const controller = globalThis.{CONTROLLER_NAME}; if (!controller) throw new Error('Page binding controller is unavailable'); return controller.take(arg); }}"
                ),
                argument,
            ),
        )
        .await
    }

    pub async fn deliver_page_binding_value(
        &self,
        call_id: u64,
        payload: EvaluationValue,
    ) -> Result<(), HostError> {
        let installation = self.page_binding_installation()?;
        let delivery = installation.begin_delivery(call_id)?;
        let source = delivery_source(&delivery.call)?;
        self.frame_evaluate(
            &delivery.call.frame.get().to_string(),
            EvaluateParams::function(source, payload),
        )
        .await
        .map(|_| ())
    }

    pub async fn deliver_page_binding_owned(
        &self,
        call_id: u64,
        payload: OwnedHandleArgument,
    ) -> Result<(), HostError> {
        let installation = self.page_binding_installation()?;
        let delivery = installation.begin_delivery(call_id)?;
        let source = delivery_source(&delivery.call)?;
        self.frame_evaluate_owned(
            &delivery.call.frame.get().to_string(),
            source,
            true,
            payload,
        )
        .await
        .map(|_| ())
    }
}

fn delivery_source(call: &PendingOperation) -> Result<String, HostError> {
    let name = serde_json::to_string(call.name.as_ref()).map_err(HostError::backend)?;
    Ok(format!(
        "payload => {{ const controller = globalThis.{CONTROLLER_NAME}; if (!controller) throw new Error('Page binding controller is unavailable'); controller.deliver({{ name: {name}, seq: {}, ...payload }}); }}",
        call.sequence
    ))
}

fn publication_source(call: &PendingOperation) -> Result<String, HostError> {
    let name = serde_json::to_string(call.name.as_ref()).map_err(HostError::backend)?;
    Ok(format!(
        "(() => {{ const controller = globalThis.{CONTROLLER_NAME}; if (!controller) throw new Error('Page binding controller is unavailable'); controller.publish({{ name: {name}, seq: {} }}); }})()",
        call.sequence
    ))
}

fn binding_call_id(call_id: &str) -> Result<u64, HostError> {
    call_id
        .parse::<u64>()
        .ok()
        .filter(|id| *id != 0)
        .ok_or_else(|| HostError::InvalidArgument("invalid binding call ID".into()))
}

impl HostSession {
    pub fn parse_page_binding_call_id(call_id: &str) -> Result<u64, HostError> {
        binding_call_id(call_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pwright_backend_cdp_topology::{
        CdpContextOwnership, CdpPageObjectIds, CdpPageTarget, CdpTopologyLimits,
        CdpWorldAttachment, CdpWorldKind, ConnectionEpoch,
    };

    fn binding_event(name: &str, payload: Value) -> CdpEvent {
        CdpEvent::new(
            "Runtime.bindingCalled".into(),
            serde_json::json!({
                "name": name,
                "payload": payload,
                "executionContextId": 17,
            }),
            Some("session-1".into()),
        )
    }

    #[test]
    fn pending_call_budget_covers_owner_and_descriptor_name_copies() {
        // Arrange
        let other_storage = std::mem::size_of::<Installation>()
            + std::mem::size_of::<State>()
            + MAX_BINDINGS * (std::mem::size_of::<Registration>() + MAX_BINDING_NAME_BYTES)
            + MAX_EARLY_CALLS
                * (std::mem::size_of::<EarlyCall>()
                    + MAX_SESSION_ID_BYTES
                    + MAX_BINDING_PAYLOAD_BYTES)
            + MAX_REJECTION_TASKS * MAX_REJECTION_TASK_BYTES
            + MAX_BINDING_SESSIONS
                * (std::mem::size_of::<Arc<SessionInstallation>>()
                    + std::mem::size_of::<SessionInstallation>()
                    + MAX_SESSION_ID_BYTES
                    + MAX_BINDINGS
                        * (std::mem::size_of::<InstalledScript>() + MAX_SCRIPT_IDENTIFIER_BYTES));
        let pending_storage = MAX_PENDING_CALLS
            * (std::mem::size_of::<PendingCall>()
                + std::mem::size_of::<BindingCall>()
                + 2 * MAX_BINDING_NAME_BYTES
                + MAX_ARGUMENT_BYTES);

        // Act
        let declared = requirements().class(ResourceClass::BackendStateBytes);

        // Assert
        assert_eq!(
            declared.global - other_storage as u64,
            pending_storage as u64
        );
        assert_eq!(declared.per_scope, declared.global);
    }

    #[test]
    fn duplicate_page_binding_registration_is_rejected_without_consuming_capacity() {
        // Arrange
        let mut state = State::empty();
        let page = ObjectId::new(7);
        let scope = RegistrationScope::Page {
            page,
            context: ObjectId::new(2),
        };
        add_registration_to_state(&mut state, scope, "add", false).unwrap();

        // Act
        let error = add_registration_to_state(&mut state, scope, "add", true).unwrap_err();

        // Assert
        assert!(error.to_string().contains("already registered"));
        assert_eq!(state.registrations.len(), 1);
        assert_eq!(state.registrations[0].name.as_ref(), "add");
    }

    #[test]
    fn context_binding_applies_to_every_page_in_its_context() {
        // Arrange
        let context = ObjectId::new(4);
        let scope = RegistrationScope::Context(context);

        // Act
        let first = scope.applies_to(ObjectId::new(10), context);
        let later = scope.applies_to(ObjectId::new(11), context);
        let unrelated = scope.applies_to(ObjectId::new(12), ObjectId::new(5));

        // Assert
        assert!(first);
        assert!(later);
        assert!(!unrelated);
    }

    #[test]
    fn reload_publication_selection_uses_exact_page_or_context_scope() {
        // Arrange
        let context = ObjectId::new(4);
        let page = ObjectId::new(10);
        let other_page = ObjectId::new(11);
        let other_context = ObjectId::new(5);
        let mut state = State::empty();
        add_registration_to_state(
            &mut state,
            RegistrationScope::Page { page, context },
            "local",
            false,
        )
        .unwrap();
        add_registration_to_state(
            &mut state,
            RegistrationScope::Context(other_context),
            "other",
            false,
        )
        .unwrap();

        // Act
        let selected_page = has_registration_for_page(&state, page, context);
        let unselected_page = has_registration_for_page(&state, other_page, context);
        let selected_context_page = has_registration_for_page(&state, other_page, other_context);
        let unselected_context = has_registration_for_page(&state, other_page, ObjectId::new(6));

        // Assert
        assert!(selected_page);
        assert!(!unselected_page);
        assert!(selected_context_page);
        assert!(!unselected_context);
    }

    #[test]
    fn publication_drain_requires_registered_or_in_flight_binding_work() {
        // Arrange
        let mut state = State::empty();

        // Act
        let empty = has_publication_sources(&state);
        add_registration_to_state(
            &mut state,
            RegistrationScope::Page {
                page: ObjectId::new(1),
                context: ObjectId::new(2),
            },
            "example",
            false,
        )
        .unwrap();
        let registered = has_publication_sources(&state);
        state.registrations.clear();
        state.pending.push(pending_call(1, false));
        let pending = has_publication_sources(&state);

        // Assert
        assert!(!empty);
        assert!(registered);
        assert!(pending);
    }

    #[test]
    fn registration_liveness_rejects_a_context_closed_after_initial_lookup() {
        // Arrange
        let epoch = ConnectionEpoch::new(1);
        let context = ObjectId::new(2);
        let mut topology = CdpTopology::new(
            ObjectId::new(1),
            epoch,
            CdpTopologyLimits::new(32, 4, 4, 8, 4, 64),
        )
        .unwrap();
        topology
            .register_context(epoch, "owned", context, CdpContextOwnership::Owned)
            .unwrap();
        let scope = RegistrationScope::Context(context);
        assert!(scope.is_live(&topology));

        // Act: the entrypoint's old snapshot survives native Context disposal.
        topology.close_context(epoch, "owned").unwrap();
        let can_register = scope.is_live(&topology);

        // Assert: registry insertion under the host-state lock must reject it.
        assert!(!can_register);
    }

    #[test]
    fn page_registration_liveness_requires_its_exact_context_membership() {
        // Arrange
        let epoch = ConnectionEpoch::new(1);
        let context = ObjectId::new(2);
        let page = ObjectId::new(3);
        let mut topology = CdpTopology::new(
            ObjectId::new(1),
            epoch,
            CdpTopologyLimits::new(32, 4, 4, 8, 4, 64),
        )
        .unwrap();
        topology
            .register_context(epoch, "owned", context, CdpContextOwnership::Owned)
            .unwrap();
        topology
            .attach_page(
                epoch,
                CdpPageTarget::new("page", "session", "owned"),
                CdpPageObjectIds::new(page, ObjectId::new(4)),
            )
            .unwrap();

        // Act
        let exact = RegistrationScope::Page { page, context }.is_live(&topology);
        let wrong_page = RegistrationScope::Page {
            page: ObjectId::new(9),
            context,
        }
        .is_live(&topology);
        topology.close_context(epoch, "owned").unwrap();
        let retired = RegistrationScope::Page { page, context }.is_live(&topology);

        // Assert
        assert!(exact);
        assert!(!wrong_page);
        assert!(!retired);
    }

    #[test]
    fn initializer_snapshot_includes_context_and_only_matching_page_bindings() {
        // Arrange
        let context = ObjectId::new(4);
        let future_page = ObjectId::new(11);
        let other_page = ObjectId::new(12);
        let mut state = State::empty();
        add_registration_to_state(
            &mut state,
            RegistrationScope::Context(context),
            "shared",
            false,
        )
        .unwrap();
        add_registration_to_state(
            &mut state,
            RegistrationScope::Page {
                page: future_page,
                context,
            },
            "local",
            true,
        )
        .unwrap();
        add_registration_to_state(
            &mut state,
            RegistrationScope::Page {
                page: other_page,
                context,
            },
            "unrelated",
            false,
        )
        .unwrap();

        // Act
        let selected = registrations_for_target(&state, future_page, context);

        // Assert
        assert_eq!(
            selected,
            vec![("shared".into(), false), ("local".into(), true)]
        );
    }

    #[test]
    fn page_binding_conflicts_with_context_binding_by_exact_playwright_message() {
        // Arrange
        let mut state = State::empty();
        let context = ObjectId::new(2);
        add_registration_to_state(
            &mut state,
            RegistrationScope::Context(context),
            "foo",
            false,
        )
        .unwrap();

        // Act
        let error = add_registration_to_state(
            &mut state,
            RegistrationScope::Page {
                page: ObjectId::new(7),
                context,
            },
            "foo",
            false,
        )
        .unwrap_err();

        // Assert
        assert_eq!(
            error.to_string(),
            "Function \"foo\" has been already registered in the browser context"
        );
    }

    #[test]
    fn context_binding_conflicts_with_a_page_binding_by_exact_playwright_message() {
        // Arrange
        let mut state = State::empty();
        let context = ObjectId::new(2);
        add_registration_to_state(
            &mut state,
            RegistrationScope::Page {
                page: ObjectId::new(7),
                context,
            },
            "baz",
            false,
        )
        .unwrap();

        // Act
        let error = add_registration_to_state(
            &mut state,
            RegistrationScope::Context(context),
            "baz",
            false,
        )
        .unwrap_err();

        // Assert
        assert_eq!(
            error.to_string(),
            "Function \"baz\" has been already registered in one of the pages"
        );
    }

    #[test]
    fn separate_pages_may_reuse_a_page_binding_name() {
        // Arrange
        let mut state = State::empty();
        let context = ObjectId::new(2);
        add_registration_to_state(
            &mut state,
            RegistrationScope::Page {
                page: ObjectId::new(7),
                context,
            },
            "sum",
            false,
        )
        .unwrap();

        // Act
        let result = add_registration_to_state(
            &mut state,
            RegistrationScope::Page {
                page: ObjectId::new(8),
                context,
            },
            "sum",
            true,
        );

        // Assert
        assert!(result.is_ok());
        assert_eq!(state.registrations.len(), 2);
    }

    #[test]
    fn page_retirement_releases_bounded_binding_capacity() {
        // Arrange
        let context = ObjectId::new(2);
        let mut state = State::empty();
        for index in 0..MAX_BINDINGS {
            add_registration_to_state(
                &mut state,
                RegistrationScope::Page {
                    page: ObjectId::new(index as u64 + 1),
                    context,
                },
                "name",
                false,
            )
            .unwrap();
        }
        assert!(matches!(
            add_registration_to_state(
                &mut state,
                RegistrationScope::Context(ObjectId::new(3)),
                "another",
                false,
            ),
            Err(HostError::ResourceExhausted(_))
        ));

        // Act
        remove_page_from_state(&mut state, ObjectId::new(1));
        let result = add_registration_to_state(
            &mut state,
            RegistrationScope::Context(ObjectId::new(3)),
            "another",
            false,
        );

        // Assert
        assert!(result.is_ok());
        assert_eq!(state.registrations.len(), MAX_BINDINGS);
    }

    #[test]
    fn context_retirement_removes_its_context_and_page_bindings() {
        // Arrange
        let context = ObjectId::new(2);
        let other = ObjectId::new(3);
        let mut state = State::empty();
        add_registration_to_state(
            &mut state,
            RegistrationScope::Context(context),
            "shared",
            false,
        )
        .unwrap();
        add_registration_to_state(
            &mut state,
            RegistrationScope::Page {
                page: ObjectId::new(7),
                context,
            },
            "local",
            false,
        )
        .unwrap();
        add_registration_to_state(
            &mut state,
            RegistrationScope::Context(other),
            "other",
            false,
        )
        .unwrap();

        // Act
        remove_context_from_state(&mut state, context);

        // Assert
        assert_eq!(state.registrations.len(), 1);
        assert_eq!(
            state.registrations[0].scope,
            RegistrationScope::Context(other)
        );
    }

    #[test]
    fn topology_reconciliation_retires_closed_context_but_keeps_reconnecting_page() {
        // Arrange
        let current = ObjectId::new(2);
        let closed = ObjectId::new(3);
        let page = ObjectId::new(7);
        let mut state = State::empty();
        add_registration_to_state(
            &mut state,
            RegistrationScope::Page {
                page,
                context: current,
            },
            "survives",
            false,
        )
        .unwrap();
        add_registration_to_state(
            &mut state,
            RegistrationScope::Context(closed),
            "retired",
            false,
        )
        .unwrap();

        // Act
        retain_materialized_registrations(&mut state, &[(page, current)], &[current]);

        // Assert
        assert_eq!(state.registrations.len(), 1);
        assert_eq!(state.registrations[0].name.as_ref(), "survives");
    }

    #[tokio::test]
    async fn context_registration_after_initializer_snapshot_reaches_unpublished_session() {
        // Arrange: initializer has published its record, but the Page has not
        // appeared in any topology route yet. Its first registration snapshot
        // is empty and it still owns the per-session script gate.
        let context = ObjectId::new(2);
        let page = ObjectId::new(7);
        let session = test_session("pending-session", page, context);
        let initializer_gate = session.scripts.clone().lock_owned().await;
        let mut state = State::empty();
        state.sessions.push(session.clone());
        assert!(registrations_for_target(&state, page, context).is_empty());

        // Act: registration observes the in-flight record even though there
        // is no published Page route. It must wait for base setup to release
        // that gate before installing its own name.
        let id = add_registration_to_state(
            &mut state,
            RegistrationScope::Context(context),
            "shared",
            false,
        )
        .unwrap();
        let records = session_records_for_scope(&state, RegistrationScope::Context(context));
        assert_eq!(records.len(), 1);
        let lock = records[0].scripts.lock();
        tokio::pin!(lock);
        assert!(poll_once(lock.as_mut()).await.is_none());
        drop(initializer_gate);
        let _registration_gate = lock.await;

        // Assert: after initializer setup, the waiting registration can
        // install the exact generation missed by the earlier snapshot.
        assert_eq!(records[0].session_id.as_ref(), "pending-session");
        assert_eq!(state.registrations[0].id, id);
        assert!(!records[0].retired.load(Ordering::Acquire));
    }

    #[test]
    fn context_registration_before_session_publication_is_in_initializer_snapshot() {
        // Arrange
        let context = ObjectId::new(2);
        let page = ObjectId::new(7);
        let mut state = State::empty();
        add_registration_to_state(
            &mut state,
            RegistrationScope::Context(context),
            "shared",
            false,
        )
        .unwrap();

        // Act: the initializer publishes its record after registration.
        state
            .sessions
            .push(test_session("later-session", page, context));
        let snapshot = registrations_for_target(&state, page, context);

        // Assert
        assert_eq!(snapshot, vec![("shared".into(), false)]);
    }

    #[tokio::test]
    async fn page_retirement_drains_pending_calls_and_unblocks_publication_fence() {
        // Arrange
        let page = ObjectId::new(7);
        let other_page = ObjectId::new(8);
        let mut first = pending_call(1, false);
        first.page = page;
        let mut second = pending_call(2, false);
        second.page = other_page;
        second.published = true;
        let state = Mutex::new(State {
            next_call: 3,
            pending: vec![first, second],
            sessions: vec![test_session("closing", page, ObjectId::new(1))],
            ..State::empty()
        });
        let changed = Notify::new();
        let stopped = CancellationToken::new();
        let wait = wait_for_publications(&state, &changed, 2, &stopped);
        tokio::pin!(wait);
        assert!(poll_once(wait.as_mut()).await.is_none());

        // Act
        remove_page_from_state(&mut state.lock().unwrap(), page);
        changed.notify_waiters();

        // Assert
        assert!(wait.await.is_ok());
        let state = state.lock().unwrap();
        assert!(!call_is_pending(&state, 1));
        assert!(call_is_pending(&state, 2));
        assert!(state.sessions.is_empty());
    }

    fn test_session(
        session_id: &str,
        page: ObjectId,
        context: ObjectId,
    ) -> Arc<SessionInstallation> {
        Arc::new(SessionInstallation {
            session_id: session_id.into(),
            page,
            context,
            initializing: AtomicBool::new(true),
            retired: AtomicBool::new(false),
            scripts: Arc::new(tokio::sync::Mutex::new(Vec::new())),
        })
    }

    fn early_call_for_session(
        session: &Arc<SessionInstallation>,
        execution_context_id: i64,
    ) -> EarlyCall {
        EarlyCall {
            session: session.clone(),
            session_id: session.session_id.clone(),
            execution_context_id,
            context: session.context,
            page: session.page,
            registration_id: 1,
            payload: "{}".into(),
        }
    }

    #[tokio::test]
    async fn cancelled_initializer_releases_its_session_and_early_call_capacity() {
        // Arrange
        let context = ObjectId::new(2);
        let page = ObjectId::new(3);
        let abandoned = test_session("abandoned", page, context);
        let mut state = State::empty();
        state.sessions.push(abandoned.clone());
        for index in 1..MAX_BINDING_SESSIONS {
            state
                .sessions
                .push(test_session(&format!("other-{index}"), page, context));
        }
        for index in 0..MAX_EARLY_CALLS {
            state
                .early
                .push(early_call_for_session(&abandoned, index as i64));
        }
        let state = Mutex::new(state);
        let mut initialization = Box::pin(async {
            let _guard = SessionInitializationGuard::new(&state, abandoned.clone());
            std::future::pending::<()>().await;
        });
        assert!(poll_once(initialization.as_mut()).await.is_none());

        // Act
        drop(initialization);

        // Assert
        let mut state = state.lock().unwrap();
        assert!(abandoned.retired.load(Ordering::Acquire));
        assert_eq!(state.sessions.len(), MAX_BINDING_SESSIONS - 1);
        assert!(state.early.is_empty());
        state
            .sessions
            .push(test_session("replacement", page, context));
        assert_eq!(state.sessions.len(), MAX_BINDING_SESSIONS);
        let retained = state.sessions[0].clone();
        state.early.push(early_call_for_session(&retained, 17));
        assert_eq!(state.early.len(), 1);
    }

    #[tokio::test]
    async fn cancelled_initializer_keeps_newer_record_with_the_same_session_id() {
        // Arrange
        let context = ObjectId::new(2);
        let page = ObjectId::new(3);
        let abandoned = test_session("reused", page, context);
        let replacement = test_session("reused", page, context);
        let mut initial = State::empty();
        initial.sessions.push(abandoned.clone());
        initial.early.push(early_call_for_session(&abandoned, 17));
        let state = Mutex::new(initial);
        let mut initialization = Box::pin(async {
            let _guard = SessionInitializationGuard::new(&state, abandoned.clone());
            std::future::pending::<()>().await;
        });
        assert!(poll_once(initialization.as_mut()).await.is_none());
        {
            let mut state = state.lock().unwrap();
            state
                .sessions
                .retain(|session| !Arc::ptr_eq(session, &abandoned));
            state.sessions.push(replacement.clone());
            state.early.push(early_call_for_session(&replacement, 18));
        }

        // Act
        drop(initialization);

        // Assert
        let state = state.lock().unwrap();
        assert!(abandoned.retired.load(Ordering::Acquire));
        assert!(!replacement.retired.load(Ordering::Acquire));
        assert_eq!(state.sessions.len(), 1);
        assert!(Arc::ptr_eq(&state.sessions[0], &replacement));
        assert_eq!(state.early.len(), 1);
        assert!(Arc::ptr_eq(&state.early[0].session, &replacement));
    }

    #[test]
    fn binding_source_quotes_untrusted_property_name() {
        // Arrange
        let name = "x']; globalThis.compromised = true; //";

        // Act
        let source = Installation::binding_source(name, true).unwrap();

        // Assert
        assert!(
            source.contains("controller__?.add(\"x']; globalThis.compromised = true; //\", true)")
        );
        assert!(!source.contains("add('x']"));
    }

    #[test]
    fn controller_tracks_publication_separately_from_callback_delivery() {
        // Arrange
        let source = CONTROLLER_SOURCE;

        // Act
        let has_publication_handshake = source.contains("drainPublications()")
            && source.contains("publish(payload)")
            && source.contains("publications.set(key, publication)");

        // Assert
        assert!(has_publication_handshake);
        assert!(source.contains("const promise = new PromiseConstructor"));
    }

    #[test]
    fn controller_rejects_local_overload_before_allocating_a_callback_or_publication() {
        // Arrange
        let source = CONTROLLER_SOURCE;

        // Act
        let guard = source.find("if (callbacks.size >= 64)").unwrap();
        let callback = source.find("callbacks.set(key").unwrap();
        let publication = source.find("publications.set(key").unwrap();

        // Assert
        assert_eq!(MAX_PENDING_CALLS, 64);
        assert!(guard < callback && guard < publication);
        assert!(source.contains("return new PromiseConstructor((_, reject) => reject(new ErrorConstructor('pending Page binding capacity exceeded (64)')))") );
        assert!(source.contains("if (utf8Length(stringify(serializedArgs)) > 65536)"));
        assert!(source.contains("if (utf8Length(encoded) > 65536 + 256 + 512)"));
    }

    #[test]
    fn rejected_call_source_quotes_page_controlled_values_and_rejects_the_promise() {
        // Arrange
        let name = "x'}); globalThis.compromised = true; //";
        let error = "full'); globalThis.compromised = true; //";

        // Act
        let source = rejection_source(name, 65, error).unwrap();

        // Assert
        assert!(source.contains("controller__?.deliver({name:\"x'}); globalThis.compromised = true; //\",seq:65,error:new Error(\"full'); globalThis.compromised = true; //\")})"));
        assert!(!source.contains("name:x'}"));
    }

    #[test]
    fn only_per_call_admission_errors_are_nonterminal() {
        // Arrange
        let invalid = HostError::InvalidArgument("bad call".into());
        let full = HostError::ResourceExhausted("full".into());
        let backend = HostError::Backend("poisoned state".into());

        // Act
        let invalid_is_call = is_per_call_rejection(&invalid);
        let full_is_call = is_per_call_rejection(&full);
        let backend_is_call = is_per_call_rejection(&backend);

        // Assert
        assert!(invalid_is_call);
        assert!(full_is_call);
        assert!(!backend_is_call);
    }

    #[test]
    fn publication_source_quotes_untrusted_binding_name() {
        // Arrange
        let call = PendingOperation {
            frame: ObjectId::new(7),
            name: "x }); globalThis.compromised = true; //".into(),
            sequence: 41,
        };

        // Act
        let source = publication_source(&call).unwrap();

        // Assert
        assert!(
            source.contains(
                "publish({ name: \"x }); globalThis.compromised = true; //\", seq: 41 })"
            )
        );
        assert!(!source.contains("name: x })"));
    }

    #[test]
    fn unrelated_cdp_binding_is_ignored_without_parsing_its_payload() {
        // Arrange
        let event = binding_event("another_library", Value::String("not-json".into()));

        // Act
        let observed = observe_browser_call(&event).unwrap();

        // Assert
        assert!(observed.is_none());
    }

    #[test]
    fn own_cdp_binding_rejects_a_malformed_payload() {
        // Arrange
        let event = binding_event(BRIDGE_NAME, Value::String("not-json".into()));

        // Act
        let error = observe_browser_call(&event).err().unwrap();

        // Assert
        assert!(error.to_string().contains("expected ident"));
    }

    #[test]
    fn malformed_own_binding_call_is_dropped_at_event_boundary() {
        // Arrange
        let event = binding_event(BRIDGE_NAME, Value::String("not-json".into()));

        // Act
        let observed = observe_browser_call_nonfatal(&event).unwrap();

        // Assert
        assert!(observed.is_none());
    }

    #[test]
    fn oversized_own_binding_call_is_dropped_at_event_boundary() {
        // Arrange
        let event = binding_event(
            BRIDGE_NAME,
            Value::String("x".repeat(MAX_ARGUMENT_BYTES + MAX_BINDING_NAME_BYTES + 513)),
        );

        // Act
        let observed = observe_browser_call_nonfatal(&event).unwrap();

        // Assert
        assert!(observed.is_none());
    }

    #[test]
    fn own_cdp_binding_accepts_the_versioned_payload() {
        // Arrange
        let event = binding_event(
            BRIDGE_NAME,
            Value::String(
                serde_json::json!({
                    "schema": BRIDGE_SCHEMA,
                    "name": "sum",
                    "seq": 4,
                    "serializedArgs": [1, 2],
                })
                .to_string(),
            ),
        );

        // Act
        let observed = observe_browser_call(&event).unwrap().unwrap();

        // Assert
        assert_eq!(observed.session_id, "session-1");
        assert_eq!(observed.execution_context_id, 17);
        assert!(observed.payload.contains("\"name\":\"sum\""));
        assert_eq!(observed.call.name, "sum");
        assert_eq!(observed.call.seq, 4);
        assert_eq!(
            observed.call.serialized_args,
            Some(serde_json::json!([1, 2]))
        );
    }

    #[test]
    fn early_fake_cdp_binding_waits_for_its_exact_main_world_route() {
        // Arrange: CDP has published the page session but not yet delivered
        // Runtime.executionContextCreated for its document-start call.
        let epoch = ConnectionEpoch::new(1);
        let context = ObjectId::new(2);
        let page = ObjectId::new(3);
        let frame = ObjectId::new(4);
        let mut topology = CdpTopology::new(
            ObjectId::new(1),
            epoch,
            CdpTopologyLimits::new(32, 4, 4, 8, 4, 64),
        )
        .unwrap();
        topology
            .register_context(epoch, "owned", context, CdpContextOwnership::Owned)
            .unwrap();
        topology
            .attach_page(
                epoch,
                CdpPageTarget::new("page", "session-1", "owned"),
                CdpPageObjectIds::new(page, frame),
            )
            .unwrap();
        let event = binding_event(
            BRIDGE_NAME,
            Value::String(
                serde_json::json!({
                    "schema": BRIDGE_SCHEMA,
                    "name": "woof",
                    "seq": 1,
                    "serializedArgs": ["context"],
                })
                .to_string(),
            ),
        );
        let observed = observe_browser_call(&event).unwrap().unwrap();
        let mut state = State::empty();
        state
            .sessions
            .push(test_session("session-1", page, context));
        let registration_id = add_registration_to_state(
            &mut state,
            RegistrationScope::Context(context),
            "woof",
            false,
        )
        .unwrap();

        // Act: deliver the binding event first, then the matching main-world
        // announcement, as the topology event driver allows.
        assert!(binding_route(&topology, "session-1", 17).is_none());
        assert!(stage_early_call(&mut state, &observed).unwrap());
        assert!(take_ready_early_call(&mut state, &topology).is_none());
        topology
            .attach_world(
                epoch,
                CdpWorldAttachment::new("session-1", 17, "main-world", "page", CdpWorldKind::Main),
            )
            .unwrap();
        let ready = take_ready_early_call(&mut state, &topology).unwrap();

        // Assert: replay is scoped to the original Context, Page, session,
        // execution context, and registration generation.
        assert_eq!(
            binding_route(&topology, "session-1", 17),
            Some((context, page, frame))
        );
        assert_eq!(ready.context, context);
        assert_eq!(ready.page, page);
        assert_eq!(ready.session_id.as_ref(), "session-1");
        assert_eq!(ready.execution_context_id, 17);
        assert_eq!(ready.registration_id, registration_id);
        assert_eq!(
            serde_json::from_str::<BrowserCall>(&ready.payload)
                .unwrap()
                .name,
            "woof"
        );
        assert!(state.early.is_empty());
    }

    #[test]
    fn context_retirement_discards_early_document_binding() {
        // Arrange
        let context = ObjectId::new(2);
        let page = ObjectId::new(3);
        let mut state = State::empty();
        state
            .sessions
            .push(test_session("session-1", page, context));
        add_registration_to_state(
            &mut state,
            RegistrationScope::Context(context),
            "woof",
            false,
        )
        .unwrap();
        let event = binding_event(
            BRIDGE_NAME,
            Value::String(
                serde_json::json!({
                    "schema": BRIDGE_SCHEMA,
                    "name": "woof",
                    "seq": 1,
                    "serializedArgs": [],
                })
                .to_string(),
            ),
        );
        let observed = observe_browser_call(&event).unwrap().unwrap();
        assert!(stage_early_call(&mut state, &observed).unwrap());

        // Act
        remove_context_from_state(&mut state, context);

        // Assert
        assert!(state.early.is_empty());
        assert!(state.sessions.is_empty());
        assert!(state.registrations.is_empty());
    }

    #[test]
    fn unowned_session_cannot_fill_early_binding_queue() {
        // Arrange
        let mut state = State::empty();
        add_registration_to_state(
            &mut state,
            RegistrationScope::Context(ObjectId::new(2)),
            "woof",
            false,
        )
        .unwrap();
        let event = binding_event(
            BRIDGE_NAME,
            Value::String(
                serde_json::json!({
                    "schema": BRIDGE_SCHEMA,
                    "name": "woof",
                    "seq": 1,
                    "serializedArgs": [],
                })
                .to_string(),
            ),
        );
        let observed = observe_browser_call(&event).unwrap().unwrap();

        // Act
        let stored = stage_early_call(&mut state, &observed).unwrap();

        // Assert
        assert!(!stored);
        assert!(state.early.is_empty());
    }

    #[test]
    fn early_binding_queue_has_a_finite_global_capacity() {
        // Arrange
        let context = ObjectId::new(2);
        let page = ObjectId::new(3);
        let mut state = State::empty();
        state
            .sessions
            .push(test_session("session-1", page, context));
        add_registration_to_state(
            &mut state,
            RegistrationScope::Context(context),
            "woof",
            false,
        )
        .unwrap();
        let event = binding_event(
            BRIDGE_NAME,
            Value::String(
                serde_json::json!({
                    "schema": BRIDGE_SCHEMA,
                    "name": "woof",
                    "seq": 1,
                    "serializedArgs": [],
                })
                .to_string(),
            ),
        );
        let observed = observe_browser_call(&event).unwrap().unwrap();
        for _ in 0..MAX_EARLY_CALLS {
            assert!(stage_early_call(&mut state, &observed).unwrap());
        }

        // Act
        let excess = stage_early_call(&mut state, &observed);

        // Assert
        assert!(matches!(excess, Err(HostError::ResourceExhausted(_))));
        assert_eq!(state.early.len(), MAX_EARLY_CALLS);
    }

    #[test]
    fn execution_context_destruction_discards_only_its_early_calls() {
        // Arrange
        let context = ObjectId::new(2);
        let page = ObjectId::new(3);
        let mut state = State::empty();
        let first_session = test_session("session-1", page, context);
        let second_session = test_session("session-2", page, context);
        state.early = vec![
            early_call_for_session(&first_session, 17),
            early_call_for_session(&first_session, 18),
            early_call_for_session(&second_session, 17),
        ];
        let event = CdpEvent::new(
            "Runtime.executionContextDestroyed".into(),
            serde_json::json!({ "executionContextId": 17 }),
            Some("session-1".into()),
        );

        // Act
        discard_destroyed_world_from_state(&mut state, &event);

        // Assert
        assert_eq!(state.early.len(), 2);
        assert_eq!(state.early[0].session_id.as_ref(), "session-1");
        assert_eq!(state.early[0].execution_context_id, 18);
        assert_eq!(state.early[1].session_id.as_ref(), "session-2");
        assert_eq!(state.early[1].execution_context_id, 17);
    }

    #[tokio::test]
    async fn publication_fence_waits_for_every_call_through_its_watermark() {
        // Arrange
        let state = Mutex::new(State {
            next_call: 3,
            pending: vec![pending_call(1, false), pending_call(2, false)],
            ..State::empty()
        });
        let changed = Notify::new();
        let stopped = CancellationToken::new();
        let wait = wait_for_publications(&state, &changed, 2, &stopped);
        tokio::pin!(wait);

        // Act
        assert!(poll_once(wait.as_mut()).await.is_none());
        state.lock().unwrap().pending[0].published = true;
        changed.notify_waiters();
        assert!(poll_once(wait.as_mut()).await.is_none());
        state.lock().unwrap().pending[1].published = true;
        changed.notify_waiters();

        // Assert
        assert!(wait.await.is_ok());
    }

    fn pending_call(id: u64, published: bool) -> PendingCall {
        PendingCall {
            id,
            context: ObjectId::new(1),
            page: ObjectId::new(1),
            frame: ObjectId::new(1),
            name: "binding".into(),
            sequence: id,
            needs_handle: false,
            handle_taken: false,
            published,
            delivering: false,
        }
    }

    async fn poll_once<F>(future: std::pin::Pin<&mut F>) -> Option<F::Output>
    where
        F: std::future::Future,
    {
        tokio::select! {
            biased;
            result = future => Some(result),
            () = tokio::task::yield_now() => None,
        }
    }
}
