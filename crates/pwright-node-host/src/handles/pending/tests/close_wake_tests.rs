use super::*;
use std::{
    future::Future,
    sync::{TryLockError, Weak},
    task::{Context, Wake, Waker},
};

#[derive(Debug)]
struct Observation {
    state_closed: Option<bool>,
    admission_rejected: Option<bool>,
}

struct AdmissionWake {
    owners: Weak<Owners>,
    observations: Mutex<Vec<Observation>>,
}

impl Wake for AdmissionWake {
    fn wake(self: Arc<Self>) {
        let owners = self.owners.upgrade().unwrap();
        let state_closed = match owners.state.try_lock() {
            Ok(state) => Some(state.closed),
            Err(TryLockError::Poisoned(state)) => Some(state.into_inner().closed),
            Err(TryLockError::WouldBlock) => None,
        };
        // Exercise real reentrant admission only after proving the lock is
        // available. The old implementation fails deterministically, not by
        // leaving a deadlocked test process behind.
        let admission_rejected =
            state_closed.map(|_| matches!(owners.admit(), Err(HostError::TargetClosed)));
        self.observations.lock().unwrap().push(Observation {
            state_closed,
            admission_rejected,
        });
    }
}

#[test]
fn close_wakes_waiters_outside_owner_lock_after_admission_is_sealed() {
    // Arrange
    let (owners, resources) = owners();
    let wake = Arc::new(AdmissionWake {
        owners: Arc::downgrade(&owners),
        observations: Mutex::new(Vec::new()),
    });
    let waker = Waker::from(wake.clone());
    let mut waiting = Box::pin(owners.wait());
    assert!(
        waiting
            .as_mut()
            .poll(&mut Context::from_waker(&waker))
            .is_pending()
    );

    // Act
    owners.close();

    // Assert
    assert!(
        waiting
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
            .is_ready()
    );
    let observations = wake.observations.lock().unwrap();
    assert!(!observations.is_empty());
    for observation in observations.iter() {
        assert_eq!(observation.state_closed, Some(true));
        assert_eq!(observation.admission_rejected, Some(true));
    }
    assert_eq!(owners.work.len(), 0);
    assert_eq!(owners.capacity.calls.load(Ordering::Acquire), 0);
    assert_eq!(owners.capacity.owners.load(Ordering::Acquire), 0);
    assert_eq!(resources.metrics().unwrap().active_leases, 1);
}
