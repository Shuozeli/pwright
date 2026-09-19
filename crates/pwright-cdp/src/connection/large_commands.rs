//! Explicit large outbound command admission. Ordinary commands and inbound
//! WebSocket limits remain unchanged; selected messages use bounded fragments.

use super::*;
use futures_util::Sink;
use tokio_tungstenite::tungstenite::protocol::frame::{
    Frame,
    coding::{Data, OpCode},
};

const CONTROL_BYTES: u64 = 512;
const COMMAND_OVERHEAD: u64 = 1024;

#[derive(Clone, Copy, Debug)]
pub struct LargeCommandProfile {
    scope: ResourceScope,
    capacity: usize,
    max_bytes: usize,
    fragment_bytes: usize,
    command_bytes: u64,
    state_bytes: u64,
    requirements: ResourceRequirements,
}
impl LargeCommandProfile {
    pub fn new(
        scope: ResourceScope,
        capacity: usize,
        max_bytes: usize,
        fragment_bytes: usize,
    ) -> Result<Self> {
        if capacity == 0 || max_bytes == 0 || fragment_bytes == 0 || fragment_bytes > max_bytes {
            return Err(CdpError::InvalidConfiguration(
                "invalid large-command limits",
            ));
        }
        let invalid = || CdpError::InvalidConfiguration("large-command resource overflow");
        // Geometric JSON Vec growth can overlap old and new allocations.
        // Framing storage is separately retained for the connection lifetime.
        let command_bytes = u64::try_from(max_bytes)
            .map_err(|_| invalid())?
            .checked_mul(3)
            .and_then(|bytes| bytes.checked_add(COMMAND_OVERHEAD))
            .ok_or_else(invalid)?;
        let state_bytes = u64::try_from(fragment_bytes)
            .map_err(|_| invalid())?
            .checked_add(14)
            .and_then(|bytes| bytes.checked_mul(4))
            .and_then(|bytes| bytes.checked_add(CONTROL_BYTES))
            .ok_or_else(invalid)?;
        let wire = command_bytes
            .checked_mul(u64::try_from(capacity).map_err(|_| invalid())?)
            .ok_or_else(invalid)?;
        let leases = capacity.checked_add(1).ok_or_else(invalid)?;
        Ok(Self {
            scope,
            capacity,
            max_bytes,
            fragment_bytes,
            command_bytes,
            state_bytes,
            requirements: ResourceRequirements::none()
                .with_class(ResourceClass::WireBytes, wire, wire)
                .with_class(ResourceClass::BackendStateBytes, state_bytes, state_bytes)
                .with_metadata(1, leases),
        })
    }
    pub const fn requirements(self) -> ResourceRequirements {
        self.requirements
    }
    pub const fn max_bytes(self) -> usize {
        self.max_bytes
    }
    pub const fn capacity(self) -> usize {
        self.capacity
    }
    pub const fn fragment_bytes(self) -> usize {
        self.fragment_bytes
    }
}

impl CdpConnectionProductionProfile {
    pub fn with_large_commands(mut self, profile: LargeCommandProfile) -> Result<Self> {
        if self.large_commands.is_some() {
            return Err(CdpError::InvalidConfiguration(
                "large commands already selected",
            ));
        }
        if profile.max_bytes <= self.config.max_command_bytes
            || profile.capacity > self.config.pending_command_capacity
            || profile.fragment_bytes > self.config.max_command_bytes
        {
            return Err(CdpError::InvalidConfiguration(
                "large-command profile does not fit ordinary command/slot limits",
            ));
        }
        for class in ResourceClass::ALL {
            let before = self.requirements.class(class);
            let added = profile.requirements().class(class);
            if before.global.checked_add(added.global).is_none()
                || before.per_scope.checked_add(added.per_scope).is_none()
            {
                return Err(CdpError::InvalidConfiguration(
                    "combined large-command resource overflow",
                ));
            }
        }
        if self
            .requirements
            .leases
            .checked_add(profile.requirements.leases)
            .is_none()
            || self
                .requirements
                .scopes
                .checked_add(profile.requirements.scopes)
                .is_none()
        {
            return Err(CdpError::InvalidConfiguration(
                "combined large-command metadata overflow",
            ));
        }
        self.requirements = self.requirements.combine_concurrent(profile.requirements());
        self.large_commands = Some(profile);
        Ok(self)
    }
}

struct State {
    active: AtomicUsize,
    fragments_written: AtomicU64,
    largest_fragment: AtomicUsize,
    _resource: ResourceLease,
}
pub(super) struct Pool {
    profile: LargeCommandProfile,
    ledger: ResourceLedger,
    state: Arc<State>,
}
impl Pool {
    pub(super) fn new(profile: LargeCommandProfile, ledger: &ResourceLedger) -> Result<Self> {
        if std::mem::size_of::<Self>() + std::mem::size_of::<State>() > CONTROL_BYTES as usize {
            return Err(CdpError::InvalidConfiguration(
                "large-command control state exceeds admission",
            ));
        }
        let resource = ledger.reserve(
            profile.scope,
            ResourceClass::BackendStateBytes,
            profile.state_bytes,
        )?;
        Ok(Self {
            profile,
            ledger: ledger.clone(),
            state: Arc::new(State {
                active: AtomicUsize::new(0),
                fragments_written: AtomicU64::new(0),
                largest_fragment: AtomicUsize::new(0),
                _resource: resource,
            }),
        })
    }
    fn admit(&self) -> Result<Arc<CommandLease>> {
        self.state
            .active
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |active| {
                (active < self.profile.capacity).then(|| active + 1)
            })
            .map_err(|active| CdpError::LimitExceeded {
                resource: "large commands",
                actual: active.saturating_add(1),
                limit: self.profile.capacity,
            })?;
        let mut lease = CommandLease {
            state: self.state.clone(),
            resource: None,
            fragment_bytes: self.profile.fragment_bytes,
        };
        lease.resource = Some(self.ledger.reserve(
            self.profile.scope,
            ResourceClass::WireBytes,
            self.profile.command_bytes,
        )?);
        Ok(Arc::new(lease))
    }
}
pub(super) struct CommandLease {
    state: Arc<State>,
    resource: Option<ResourceLease>,
    fragment_bytes: usize,
}
impl CommandLease {
    pub(super) fn fragment_bytes(&self) -> usize {
        self.fragment_bytes
    }
    fn record_fragment(&self, bytes: usize) {
        self.state
            .largest_fragment
            .fetch_max(bytes, Ordering::AcqRel);
        let _ = self.state.fragments_written.fetch_update(
            Ordering::AcqRel,
            Ordering::Acquire,
            |count| Some(count.saturating_add(1)),
        );
    }
}
impl Drop for CommandLease {
    fn drop(&mut self) {
        self.resource = None;
        self.state.active.fetch_sub(1, Ordering::AcqRel);
    }
}

impl CdpConnection {
    /// A lightweight, non-transactional snapshot of the selected outbound pool.
    pub fn large_command_metrics(&self) -> Option<LargeCommandMetrics> {
        self.large_commands
            .as_ref()
            .map(|pool| LargeCommandMetrics {
                active: pool.state.active.load(Ordering::Acquire),
                fragments_written: pool.state.fragments_written.load(Ordering::Acquire),
                largest_fragment_payload: pool.state.largest_fragment.load(Ordering::Acquire),
            })
    }
    /// Requires explicit production selection. Shares ordinary pending/write
    /// bounds, but also retains an independent large-message lease across raw
    /// construction, queued/writing frames, native wait and unconsumed response.
    /// Callers budget their input Value before invoking this transport API.
    pub fn send_large_command_to_settlement_with_session(
        &self,
        method: &str,
        params: Value,
        session_id: Option<&str>,
    ) -> impl Future<Output = Result<Value>> + Send + '_ {
        let prepared = self
            .large_commands
            .as_ref()
            .ok_or(CdpError::InvalidConfiguration(
                "large commands were not selected",
            ))
            .and_then(|pool| Ok((pool.profile.max_bytes, pool.admit()?)))
            .and_then(|(max_bytes, large)| {
                self.prepare_admitted_command(
                    method,
                    params,
                    session_id,
                    CommandAdmission {
                        max_bytes,
                        large: Some(large),
                    },
                )
            });
        self.send_prepared(prepared, None)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LargeCommandMetrics {
    pub active: usize,
    pub fragments_written: u64,
    pub largest_fragment_payload: usize,
}

impl crate::CdpSession {
    pub async fn send_large_to_settlement(&self, method: &str, params: Value) -> Result<Value> {
        self.connection()
            .send_large_command_to_settlement_with_session(method, params, self.session_id())
            .await
    }
}

/// UTF-8 may cross fragment boundaries; RFC 6455 reconstructs the complete text
/// message before decoding. Bytes slices share the admitted raw allocation.
/// Flush each bounded fragment rather than accumulating a full large frame in
/// tungstenite's retained writer buffer. Never interleave two data messages.
pub(super) async fn write_fragmented<S: Sink<Message> + Unpin>(
    sink: &mut S,
    message: Message,
    lease: &CommandLease,
) -> std::result::Result<(), S::Error> {
    let bytes = message.into_data();
    let mut start: usize = 0;
    loop {
        let end = start
            .saturating_add(lease.fragment_bytes())
            .min(bytes.len());
        let opcode = if start == 0 {
            Data::Text
        } else {
            Data::Continue
        };
        sink.send(Message::Frame(Frame::message(
            bytes.slice(start..end),
            OpCode::Data(opcode),
            end == bytes.len(),
        )))
        .await?;
        lease.record_fragment(end - start);
        if end == bytes.len() {
            return Ok(());
        }
        start = end;
    }
}
