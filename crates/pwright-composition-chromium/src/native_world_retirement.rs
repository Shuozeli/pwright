//! Opt-in destruction receipts for budgeted paired-world snapshots. These are
//! NOT route liveness and never infer destruction from generic detach events.
//! Explicit owned-session teardown records object retirement separately.
use crate::{CdpPageRouteError, frame_world_access::Permit};
use pwright_core::{ObjectId, ResourceLease};
use std::sync::{
    Arc, Mutex, Weak,
    atomic::{AtomicBool, AtomicU8, Ordering},
};
use std::task::{Context, Poll, Waker};

const MAIN: u8 = 1;
const UTILITY: u8 = 2;
const BOTH: u8 = MAIN | UTILITY;

pub(crate) struct Identity {
    pub epoch: pwright_backend_cdp::ConnectionEpoch,
    pub context: ObjectId,
    // Frozen owning Page target, not the local/OOPIF session target. Keep it
    // after route removal so a late real targetDestroyed event remains useful.
    pub page_target: Arc<str>,
    pub session: Arc<str>,
    // None describes a utility-only snapshot, not an unobserved main world.
    pub main: Option<Arc<str>>,
    pub utility: Arc<str>,
}
struct Receipt {
    identity: Identity,
    destroyed: AtomicU8,
    session_retired: AtomicBool,
    session_retiring: AtomicBool,
    session_attempted: AtomicBool,
    waiter: Mutex<Option<Waker>>,
    permit: Weak<Permit>,
}

/// A monotonic receipt for destruction of BOTH exact worlds on one connection.
/// The retained snapshot permit follows every clone, even after snapshot drop.
/// Only protocol destruction events and successful native-disposal acknowledgments
/// can update it. No public caller can manufacture a destroyed receipt.
#[derive(Clone)]
pub struct ChromiumNativeWorldLifetime {
    receipt: Arc<Receipt>,
    _permit: Arc<Permit>,
}
impl ChromiumNativeWorldLifetime {
    pub fn both_worlds_destroyed(&self) -> bool {
        self.receipt.destroyed.load(Ordering::Acquire) == BOTH
    }
    /// Native wrappers retired without claiming the borrowed worlds died.
    pub fn native_objects_retired(&self) -> bool {
        self.both_worlds_destroyed() || self.receipt.session_retired.load(Ordering::Acquire)
    }
    /// One evaluation waiter per receipt. Clones used for group ownership must
    /// not poll concurrently. Only acknowledged session retirement wakes this
    /// port; ordinary world destruction still settles the native CDP request.
    pub fn poll_session_retired(&mut self, cx: &mut Context<'_>) -> Poll<()> {
        let Ok(mut waiter) = self.receipt.waiter.lock() else {
            return Poll::Pending;
        };
        if self.receipt.session_retired.load(Ordering::Acquire) {
            Poll::Ready(())
        } else {
            *waiter = Some(cx.waker().clone());
            Poll::Pending
        }
    }
    /// Prevent a failed native call/release from outracing its in-flight detach
    /// acknowledgement. This is a fence only, never proof of successful detach.
    pub fn poll_session_release_ready(&self, cx: &mut Context<'_>) -> Poll<()> {
        let Ok(mut waiter) = self.receipt.waiter.lock() else {
            return Poll::Pending;
        };
        if self.receipt.session_retiring.load(Ordering::Acquire) {
            *waiter = Some(cx.waker().clone());
            Poll::Pending
        } else {
            Poll::Ready(())
        }
    }
}

pub(crate) struct Registry {
    entries: Mutex<Vec<Weak<Receipt>>>,
    capacity: usize,
    sealed: AtomicBool,
    // The route table may retain this registry after its accessor is dropped.
    // Share its installed-state lease rather than freeing that budget early.
    _lease: Arc<ResourceLease>,
}
impl Registry {
    pub(crate) fn storage_bytes(capacity: usize) -> Option<u64> {
        capacity
            // Expired Weak entries retain their Arc allocation until the next
            // registration prunes them. Charge that inline storage here too,
            // rather than to a snapshot permit which may already be released.
            .checked_mul(std::mem::size_of::<Weak<Receipt>>() + std::mem::size_of::<Receipt>() + 32)
            .and_then(|bytes| bytes.checked_add(std::mem::size_of::<Self>() + 256))
            .and_then(|bytes| u64::try_from(bytes).ok())
    }
    pub(crate) fn new(capacity: usize, lease: Arc<ResourceLease>) -> Self {
        Self {
            entries: Mutex::new(Vec::with_capacity(capacity)),
            capacity,
            sealed: AtomicBool::new(false),
            _lease: lease,
        }
    }
    pub(crate) fn register(
        &self,
        identity: Identity,
        permit: Arc<Permit>,
    ) -> Result<ChromiumNativeWorldLifetime, CdpPageRouteError> {
        let mut entries = self
            .entries
            .lock()
            .map_err(|_| CdpPageRouteError::StatePoisoned)?;
        if self.sealed.load(Ordering::Acquire) {
            return Err(CdpPageRouteError::NativeRetirementSealed);
        }
        entries.retain(|entry| entry.strong_count() > 0);
        if entries.len() >= self.capacity {
            return Err(CdpPageRouteError::CapacityExceeded {
                actual: entries.len() + 1,
                limit: self.capacity,
            });
        }
        let destroyed = if identity.main.is_none() { MAIN } else { 0 };
        let receipt = Arc::new(Receipt {
            identity,
            destroyed: AtomicU8::new(destroyed),
            session_retired: AtomicBool::new(false),
            session_retiring: AtomicBool::new(false),
            session_attempted: AtomicBool::new(false),
            waiter: Mutex::new(None),
            permit: Arc::downgrade(&permit),
        });
        entries.push(Arc::downgrade(&receipt));
        Ok(ChromiumNativeWorldLifetime {
            receipt,
            _permit: permit,
        })
    }

    pub(crate) async fn retire_sessions(
        &self,
        epoch: pwright_backend_cdp::ConnectionEpoch,
        browser: &dyn pwright_cdp::CdpClient,
    ) -> Result<(), crate::FrameWorldAccessError> {
        {
            let _entries = self
                .entries
                .lock()
                .map_err(|_| CdpPageRouteError::StatePoisoned)?;
            if self.sealed.swap(true, Ordering::AcqRel) {
                return Err(crate::FrameWorldAccessError::Setup(
                    "native retirement already sealed",
                ));
            }
        }
        let mut failed = false;
        // The sealed, pre-budgeted weak inventory cannot grow. Retain only one
        // snapshot permit across I/O; no extra session list or task is allocated.
        for index in 0..self.capacity {
            let lifetime = {
                let entries = self
                    .entries
                    .lock()
                    .map_err(|_| CdpPageRouteError::StatePoisoned)?;
                entries
                    .get(index)
                    .and_then(Weak::upgrade)
                    .and_then(|receipt| {
                        receipt
                            .permit
                            .upgrade()
                            .map(|permit| ChromiumNativeWorldLifetime {
                                receipt,
                                _permit: permit,
                            })
                    })
            };
            let Some(lifetime) = lifetime else {
                continue;
            };
            if lifetime.native_objects_retired()
                || lifetime.receipt.session_attempted.load(Ordering::Acquire)
            {
                continue;
            }
            let identity = &lifetime.receipt.identity;
            if identity.epoch != epoch {
                failed = true;
                continue;
            }
            self.begin_session(epoch, &identity.session)?;
            if browser.target_detach(&identity.session).await.is_err() {
                // No missing-session text or generic event is an ack. Continue
                // independent sessions, but never report this shutdown clean.
                failed = true;
                self.finish_session(epoch, &identity.session, false)?;
                continue;
            }
            self.finish_session(epoch, &identity.session, true)?;
        }
        if failed {
            Err(crate::FrameWorldAccessError::Setup(
                "one or more owned native sessions could not be retired",
            ))
        } else {
            Ok(())
        }
    }

    fn begin_session(
        &self,
        epoch: pwright_backend_cdp::ConnectionEpoch,
        session: &str,
    ) -> Result<(), CdpPageRouteError> {
        let entries = self
            .entries
            .lock()
            .map_err(|_| CdpPageRouteError::StatePoisoned)?;
        for receipt in entries.iter().filter_map(Weak::upgrade) {
            if receipt.identity.epoch == epoch && receipt.identity.session.as_ref() == session {
                let _waiter = receipt
                    .waiter
                    .lock()
                    .map_err(|_| CdpPageRouteError::StatePoisoned)?;
                receipt.session_retiring.store(true, Ordering::Release);
                receipt.session_attempted.store(true, Ordering::Release);
            }
        }
        Ok(())
    }

    fn finish_session(
        &self,
        epoch: pwright_backend_cdp::ConnectionEpoch,
        session: &str,
        succeeded: bool,
    ) -> Result<(), CdpPageRouteError> {
        // Wake outside BOTH locks: a caller waker may synchronously reenter.
        for index in 0..self.capacity {
            let receipt = self
                .entries
                .lock()
                .map_err(|_| CdpPageRouteError::StatePoisoned)?
                .get(index)
                .and_then(Weak::upgrade);
            let Some(receipt) = receipt else {
                continue;
            };
            if receipt.identity.epoch != epoch || receipt.identity.session.as_ref() != session {
                continue;
            }
            let waiter = {
                let mut waiter = receipt
                    .waiter
                    .lock()
                    .map_err(|_| CdpPageRouteError::StatePoisoned)?;
                receipt.session_retired.store(succeeded, Ordering::Release);
                receipt.session_retiring.store(false, Ordering::Release);
                waiter.take()
            };
            if let Some(waiter) = waiter {
                waiter.wake();
            }
        }
        Ok(())
    }
    pub(crate) fn mark(
        &self,
        epoch: pwright_backend_cdp::ConnectionEpoch,
        event: Destruction<'_>,
    ) -> Result<(), CdpPageRouteError> {
        let entries = self
            .entries
            .lock()
            .map_err(|_| CdpPageRouteError::StatePoisoned)?;
        for receipt in entries.iter().filter_map(Weak::upgrade) {
            let id = &receipt.identity;
            if id.epoch != epoch {
                continue;
            }
            let bits = match event {
                Destruction::Context(context) if id.context == context => BOTH,
                Destruction::PageTarget(target) if id.page_target.as_ref() == target => BOTH,
                Destruction::SessionWorlds(session) if id.session.as_ref() == session => BOTH,
                Destruction::World { session, unique } if id.session.as_ref() == session => {
                    if id.main.as_deref() == Some(unique) {
                        MAIN
                    } else if id.utility.as_ref() == unique {
                        UTILITY
                    } else {
                        0
                    }
                }
                _ => 0,
            };
            receipt.destroyed.fetch_or(bits, Ordering::AcqRel);
        }
        Ok(())
    }
}

#[derive(Clone, Copy)]
pub(crate) enum Destruction<'a> {
    Context(ObjectId),
    PageTarget(&'a str),
    SessionWorlds(&'a str),
    World { session: &'a str, unique: &'a str },
}

impl crate::PreparedCdpPageRoutes<'_> {
    pub(crate) fn observe_native_destruction(
        &self,
        event: Destruction<'_>,
    ) -> Result<(), CdpPageRouteError> {
        // The prepared transaction retains the same lock as snapshot
        // lookup/registration until its replacement routes are published.
        if let Some(registry) = &self.current.native_world_retirement {
            // The candidate topology identifies the operation's epoch, even
            // if a close acknowledgement races a newer connection publication.
            registry.mark(self.next.epoch.expect("prepared topology epoch"), event)?;
        }
        Ok(())
    }
}
