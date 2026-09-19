//! Optional bounded reducer for one Page's normalized Frame lifecycle stream.
//!
//! The embedding Rust owner validates native connection/session/loader identity
//! before normalization, supplies monotonic time, and owns event delivery. This
//! reducer performs no CDP I/O, owns no task, and cannot complete a navigation by
//! itself. Request ObjectIds must be unique for this observer's lifetime.
use pwright_core::{
    ObjectId, ProductionResourceLedger, ResourceClass, ResourceLease, ResourceRequirements,
    ResourceScope,
};
use std::{error::Error, fmt, mem::size_of, time::Duration};

pub mod native;
pub mod navigation;

#[cfg(test)]
mod tests;
mod transitions;

const IDLE_DELAY: Duration = Duration::from_millis(500);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LifecycleError {
    InvalidProfile,
    SizeOverflow,
    Resources,
    Allocation,
    FrameCapacity,
    RequestCapacity,
    InvalidIdentity,
    MissingParent,
    ConflictingIdentity,
    TimeWentBackwards,
    Invalidated,
}
impl fmt::Display for LifecycleError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "Frame lifecycle: {self:?}")
    }
}
impl Error for LifecycleError {}

/// Caller-normalized new-document generation, not a CDP loader ID. Each Frame's
/// generations must increase; old-generation lifecycle events are ignored.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct DocumentGeneration(pub u64);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DocumentLifecycle {
    DomContentLoaded,
    Load,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LifecycleInput {
    Attach {
        frame: ObjectId,
        parent: Option<ObjectId>,
    },
    Commit {
        frame: ObjectId,
        document: DocumentGeneration,
        request: Option<ObjectId>,
    },
    DocumentEvent {
        frame: ObjectId,
        document: DocumentGeneration,
        event: DocumentLifecycle,
    },
    RequestStarted {
        frame: ObjectId,
        request: ObjectId,
        favicon: bool,
    },
    RequestFinished {
        request: ObjectId,
    },
    Detach {
        frame: ObjectId,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LifecycleSnapshot {
    pub document: Option<DocumentGeneration>,
    pub commit: bool,
    pub dom_content_loaded: bool,
    pub load: bool,
    pub network_idle: bool,
}
impl LifecycleSnapshot {
    /// Check a lifecycle after the caller has matched the intended document.
    /// Navigation-response and interruption semantics belong to the operation.
    pub const fn reached(self, wait: pwright_cap_navigation::WaitUntil) -> bool {
        match wait {
            pwright_cap_navigation::WaitUntil::Commit => self.commit,
            pwright_cap_navigation::WaitUntil::DomContentLoaded => self.dom_content_loaded,
            pwright_cap_navigation::WaitUntil::Load => self.load,
            pwright_cap_navigation::WaitUntil::NetworkIdle => self.network_idle,
        }
    }

    fn initial() -> Self {
        Self {
            document: None,
            commit: true,
            dom_content_loaded: false,
            load: false,
            network_idle: false,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct FrameLifecycleProfile {
    scope: ResourceScope,
    frames: usize,
    requests: usize,
    bytes: u64,
}
impl FrameLifecycleProfile {
    pub fn new(
        scope: ResourceScope,
        frames: usize,
        requests: usize,
    ) -> Result<Self, LifecycleError> {
        if frames == 0 || requests == 0 {
            return Err(LifecycleError::InvalidProfile);
        }
        let bytes = size_of::<FrameLifecycleStore>()
            .checked_add(
                frames
                    .checked_mul(size_of::<Option<FrameState>>())
                    .ok_or(LifecycleError::SizeOverflow)?,
            )
            .and_then(|sum| {
                sum.checked_add(requests.checked_mul(size_of::<Option<RequestState>>())?)
            })
            .and_then(|bytes| u64::try_from(bytes).ok())
            .ok_or(LifecycleError::SizeOverflow)?;
        Ok(Self {
            scope,
            frames,
            requests,
            bytes,
        })
    }
    pub const fn requirements(self) -> ResourceRequirements {
        ResourceRequirements::none()
            .with_class(ResourceClass::BackendStateBytes, self.bytes, self.bytes)
            .with_metadata(1, 1)
    }
    pub const fn retained_bytes(self) -> u64 {
        self.bytes
    }
}

struct FrameState {
    id: ObjectId,
    parent: Option<ObjectId>,
    depth: usize,
    snapshot: LifecycleSnapshot,
    requests: usize,
    self_idle: bool,
    deadline: Option<Duration>,
    retiring: bool,
}
struct RequestState {
    id: ObjectId,
    frame: ObjectId,
}

/// Fixed-capacity storage admitted before allocation. All later transitions and
/// snapshots are allocation-free. Bounds cover logical slots/body, not allocator
/// metadata, the external normalizer, event queue or a process-wide RAM ceiling.
pub struct FrameLifecycleStore {
    frames: Vec<Option<FrameState>>,
    requests: Vec<Option<RequestState>>,
    now: Duration,
    invalidated: bool,
    _lease: ResourceLease,
}
impl FrameLifecycleStore {
    pub fn new(
        profile: FrameLifecycleProfile,
        resources: &ProductionResourceLedger,
    ) -> Result<Self, LifecycleError> {
        let ledger = resources
            .claim(profile.requirements())
            .map_err(|_| LifecycleError::Resources)?;
        let lease = ledger
            .reserve(
                profile.scope,
                ResourceClass::BackendStateBytes,
                profile.bytes,
            )
            .map_err(|_| LifecycleError::Resources)?;
        Ok(Self {
            frames: slots(profile.frames)?,
            requests: slots(profile.requests)?,
            now: Duration::ZERO,
            invalidated: false,
            _lease: lease,
        })
    }

    /// Drain elapsed timers before applying a normalized event at `now`.
    /// The owner must supply event-processing time and preserve stream order.
    /// Invalid/capacity-exhausted streams are sealed: stale snapshots cannot
    /// subsequently be mistaken for successful lifecycle completion.
    pub fn apply(&mut self, now: Duration, input: LifecycleInput) -> Result<(), LifecycleError> {
        self.advance(now)?;
        let result = self.transition(input);
        if result.is_err() {
            self.invalidate();
        }
        result
    }

    pub fn advance(&mut self, now: Duration) -> Result<(), LifecycleError> {
        if self.invalidated {
            return Err(LifecycleError::Invalidated);
        }
        if now < self.now {
            self.invalidate();
            return Err(LifecycleError::TimeWentBackwards);
        }
        self.now = now;
        let mut changed = false;
        for row in self.frames.iter_mut().flatten() {
            if row.deadline.is_some_and(|deadline| deadline <= now) {
                row.deadline = None;
                row.self_idle = true;
                changed = true;
            }
        }
        if changed {
            self.recalculate();
        }
        Ok(())
    }

    pub fn snapshot(&self, frame: ObjectId) -> Result<Option<LifecycleSnapshot>, LifecycleError> {
        if self.invalidated {
            return Err(LifecycleError::Invalidated);
        }
        Ok(self
            .frame_index(frame)
            .map(|index| self.frames[index].as_ref().unwrap().snapshot))
    }
    pub fn next_deadline(&self) -> Result<Option<Duration>, LifecycleError> {
        if self.invalidated {
            return Err(LifecycleError::Invalidated);
        }
        Ok(self
            .frames
            .iter()
            .flatten()
            .filter_map(|row| row.deadline)
            .min())
    }
    /// Call after dropped/lagged events, disconnect or normalizer failure.
    pub fn invalidate(&mut self) {
        self.invalidated = true;
    }
    fn frame_index(&self, frame: ObjectId) -> Option<usize> {
        self.frames
            .iter()
            .position(|row| row.as_ref().is_some_and(|row| row.id == frame))
    }
    fn idle_deadline(&self) -> Result<Duration, LifecycleError> {
        self.now
            .checked_add(IDLE_DELAY)
            .ok_or(LifecycleError::SizeOverflow)
    }

    fn recalculate(&mut self) {
        let max_depth = self
            .frames
            .iter()
            .flatten()
            .map(|row| row.depth)
            .max()
            .unwrap_or(0);
        // Children first. Slot/depth traversal uses no recursion or scratch map.
        for depth in (0..=max_depth).rev() {
            for index in 0..self.frames.len() {
                let Some(row) = self.frames[index].as_ref().filter(|row| row.depth == depth) else {
                    continue;
                };
                let ready = row.self_idle
                    && self
                        .frames
                        .iter()
                        .flatten()
                        .filter(|child| child.parent == Some(row.id))
                        .all(|child| child.snapshot.network_idle);
                self.frames[index].as_mut().unwrap().snapshot.network_idle = ready;
            }
        }
    }
}

fn slots<T>(count: usize) -> Result<Vec<Option<T>>, LifecycleError> {
    let mut slots = Vec::new();
    slots
        .try_reserve_exact(count)
        .map_err(|_| LifecycleError::Allocation)?;
    if slots.capacity() != count {
        return Err(LifecycleError::Allocation);
    }
    slots.resize_with(count, || None);
    Ok(slots)
}
