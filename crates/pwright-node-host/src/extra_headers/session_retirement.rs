//! Request-scoped native close watches, fed by the existing ordered cursor.
//! No session history, second subscriber, task, or default-on network domain.
use super::{ID_BYTES, Installation, MAX_REQUESTS, MAX_ROUTES};
use crate::HostError;
use futures_util::task::AtomicWaker;
use pwright_backend_cdp_topology::CdpTopology;
use pwright_cdp::{
    CdpEvent,
    generated::{page, target},
};
use pwright_core::ObjectId;
use pwright_plugin_chromium_frame_observer::{ChromiumFrameEventObserver, FrameObservationResult};
use serde::Deserialize;
use std::{
    future::Future,
    pin::Pin,
    sync::{
        Arc, Mutex, Weak,
        atomic::{AtomicBool, Ordering},
    },
    task::{Context, Poll},
};

const MAX_WATCHES: usize = MAX_REQUESTS * MAX_ROUTES;

pub(super) struct Watch {
    pub(super) session: String,
    main_session: String,
    target: String,
    closed: AtomicBool,
    // Primary and compensation may both await the same native session.
    wakers: [AtomicWaker; 2],
}
impl Watch {
    pub(super) fn new(session: &str, main_session: &str, target: &str) -> Arc<Self> {
        Arc::new(Self {
            session: session.into(),
            main_session: main_session.into(),
            target: target.into(),
            closed: AtomicBool::new(false),
            wakers: [AtomicWaker::new(), AtomicWaker::new()],
        })
    }
    pub(super) fn is_main(&self) -> bool {
        self.session == self.main_session
    }
    fn close(&self) {
        self.closed.store(true, Ordering::Release);
        for waker in &self.wakers {
            waker.wake();
        }
    }
    pub(super) fn wait(&self, compensation: bool) -> Closed<'_> {
        Closed {
            watch: self,
            slot: usize::from(compensation),
        }
    }
}
pub(super) struct Closed<'a> {
    watch: &'a Watch,
    slot: usize,
}
impl Future for Closed<'_> {
    type Output = ();
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        self.watch.wakers[self.slot].register(cx.waker());
        if self.watch.closed.load(Ordering::Acquire) {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }
}
impl Drop for Closed<'_> {
    fn drop(&mut self) {
        self.watch.wakers[self.slot].take();
    }
}

pub(super) struct Watches(Mutex<Vec<Weak<Watch>>>);
impl Watches {
    pub(super) fn new() -> Self {
        Self(Mutex::new(Vec::with_capacity(MAX_WATCHES)))
    }
    pub(super) fn insert(
        &self,
        session: &str,
        main: &str,
        target: &str,
    ) -> Result<Arc<Watch>, HostError> {
        let mut watches = self.0.lock().map_err(HostError::backend)?;
        watches.retain(|watch| watch.strong_count() != 0);
        if watches.len() == MAX_WATCHES
            || [session, main, target].iter().any(|id| id.len() > ID_BYTES)
        {
            return Err(HostError::ResourceExhausted(
                "header session watch capacity exceeded".into(),
            ));
        }
        let watch = Watch::new(session, main, target);
        watches.push(Arc::downgrade(&watch));
        Ok(watch)
    }
    fn close_matching(&self, matches: impl Fn(&Watch) -> bool) -> Result<(), HostError> {
        let mut watches = self.0.lock().map_err(HostError::backend)?;
        watches.retain(|entry| {
            let Some(watch) = entry.upgrade() else {
                return false;
            };
            if matches(&watch) {
                watch.close();
            }
            true
        });
        Ok(())
    }
}
pub(super) fn registry_bytes() -> usize {
    MAX_WATCHES * std::mem::size_of::<Weak<Watch>>()
}
pub(super) fn route_bytes() -> usize {
    std::mem::size_of::<Watch>() + 3 * ID_BYTES + 32
}

pub(crate) struct Observer {
    installation: Arc<Installation>,
    frames: Option<Arc<dyn ChromiumFrameEventObserver>>,
}
impl Observer {
    pub(crate) fn new(
        installation: Arc<Installation>,
        frames: Option<Arc<dyn ChromiumFrameEventObserver>>,
    ) -> Self {
        Self {
            installation,
            frames,
        }
    }
}
impl ChromiumFrameEventObserver for Observer {
    fn observes_page(&self, page: ObjectId) -> bool {
        self.frames
            .as_ref()
            .is_some_and(|frames| frames.observes_page(page))
    }
    fn retains_worker_targets(&self) -> bool {
        self.frames
            .as_ref()
            .is_some_and(|frames| frames.retains_worker_targets())
    }
    fn retains_worker_session(&self, session: &str) -> bool {
        self.frames
            .as_ref()
            .is_some_and(|frames| frames.retains_worker_session(session))
    }

    fn enables_worker_network(&self) -> bool {
        self.frames
            .as_ref()
            .is_some_and(|frames| frames.enables_worker_network())
    }
    fn prepare_page(&self, topology: &CdpTopology, page: ObjectId) -> FrameObservationResult {
        if let Some(frames) = &self.frames {
            frames.prepare_page(topology, page)?;
        }
        Ok(())
    }
    fn discard_prepared_page(
        &self,
        topology: &CdpTopology,
        page: ObjectId,
    ) -> FrameObservationResult {
        if let Some(frames) = &self.frames {
            frames.discard_prepared_page(topology, page)?;
        }
        Ok(())
    }
    fn initialize_frame(
        &self,
        topology: &CdpTopology,
        session: &str,
        frame: &page::Frame,
    ) -> FrameObservationResult {
        if let Some(frames) = &self.frames {
            frames.initialize_frame(topology, session, frame)?;
        }
        Ok(())
    }
    fn reconcile(&self, topology: &CdpTopology) -> FrameObservationResult {
        if let Some(frames) = &self.frames {
            frames.reconcile(topology)?;
        }
        Ok(())
    }
    fn event(&self, topology: &CdpTopology, event: &CdpEvent) -> FrameObservationResult {
        match event.method.as_str() {
            "Target.detachedFromTarget" => {
                let detached = target::DetachedFromTargetEvent::deserialize(&event.params)?;
                self.installation.watches.close_matching(|watch| {
                    watch.session == detached.session_id
                        || watch.main_session == detached.session_id
                })?;
            }
            "Target.targetCrashed" => {
                let crashed = target::TargetCrashedEvent::deserialize(&event.params)?;
                self.installation
                    .watches
                    .close_matching(|watch| watch.target == crashed.target_id)?;
            }
            _ => {}
        }
        if let Some(frames) = &self.frames {
            frames.event(topology, event)?;
        }
        Ok(())
    }
    fn seal(&self) {
        // Seal is terminal. A poisoned registry already rejects all admissions;
        // recover its entries here only to wake existing waiters for teardown.
        let watches = self
            .installation
            .watches
            .0
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        for watch in watches.iter().filter_map(Weak::upgrade) {
            watch.close();
        }
        if let Some(frames) = &self.frames {
            frames.seal();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::FutureExt;
    use std::sync::atomic::AtomicUsize;
    use std::task::{Wake, Waker};

    struct WakeCount(AtomicUsize);
    impl Wake for WakeCount {
        fn wake(self: Arc<Self>) {
            self.0.fetch_add(1, Ordering::AcqRel);
        }
    }

    #[test]
    fn native_close_wakes_primary_and_compensation_but_not_another_session() {
        // Arrange
        let watches = Watches::new();
        let child = watches.insert("child", "main", "child-target").unwrap();
        let other = watches
            .insert("other", "other-main", "other-target")
            .unwrap();
        let mut primary = Box::pin(child.wait(false));
        let mut compensation = Box::pin(child.wait(true));
        let counts = [
            Arc::new(WakeCount(AtomicUsize::new(0))),
            Arc::new(WakeCount(AtomicUsize::new(0))),
        ];
        let wakers = counts.each_ref().map(|count| Waker::from(count.clone()));
        assert!(
            primary
                .as_mut()
                .poll(&mut Context::from_waker(&wakers[0]))
                .is_pending()
        );
        assert!(
            compensation
                .as_mut()
                .poll(&mut Context::from_waker(&wakers[1]))
                .is_pending()
        );
        // Act
        watches
            .close_matching(|watch| watch.session == "child")
            .unwrap();
        // Assert
        assert_eq!(primary.now_or_never(), Some(()));
        assert_eq!(compensation.now_or_never(), Some(()));
        assert_eq!(
            counts
                .each_ref()
                .map(|count| count.0.load(Ordering::Acquire)),
            [1, 1]
        );
        assert!(other.wait(false).now_or_never().is_none());
        assert!(!child.is_main());
    }

    #[test]
    fn dropping_a_waiter_releases_its_registered_task_waker() {
        // Arrange
        let watch = Watch::new("child", "main", "target");
        let counter = Arc::new(WakeCount(AtomicUsize::new(0)));
        let waker = Waker::from(counter.clone());
        let mut waiter = Box::pin(watch.wait(false));
        assert!(
            waiter
                .as_mut()
                .poll(&mut Context::from_waker(&waker))
                .is_pending()
        );
        assert_eq!(Arc::strong_count(&counter), 3);
        // Act
        drop(waiter);
        // Assert
        assert_eq!(Arc::strong_count(&counter), 2);
        watch.close();
        assert_eq!(counter.0.load(Ordering::Acquire), 0);
    }

    #[test]
    fn main_session_close_retires_child_watches_before_their_dispatch() {
        // Arrange
        let watches = Watches::new();
        let main = watches.insert("main", "main", "page").unwrap();
        let child = watches.insert("child", "main", "frame").unwrap();
        // Act
        watches
            .close_matching(|watch| watch.session == "main" || watch.main_session == "main")
            .unwrap();
        // Assert
        assert!(main.is_main());
        assert_eq!(main.wait(false).now_or_never(), Some(()));
        assert_eq!(child.wait(false).now_or_never(), Some(()));
    }

    #[test]
    fn completed_requests_release_watch_capacity_without_retaining_session_history() {
        // Arrange
        let watches = Watches::new();
        let held: Vec<_> = (0..MAX_WATCHES)
            .map(|_| watches.insert("session", "main", "target").unwrap())
            .collect();
        assert!(matches!(
            watches.insert("excess", "main", "target"),
            Err(HostError::ResourceExhausted(_))
        ));
        // Act
        drop(held);
        let next = watches.insert("next", "main", "target").unwrap();
        // Assert
        assert_eq!(watches.0.lock().unwrap().len(), 1);
        assert_eq!(next.session, "next");
    }

    #[test]
    fn route_id_limit_rejects_before_adding_a_watch() {
        // Arrange
        let watches = Watches::new();
        let long = "x".repeat(ID_BYTES + 1);
        // Act
        let result = watches.insert("session", "main", &long);
        // Assert
        assert!(matches!(result, Err(HostError::ResourceExhausted(_))));
        assert!(watches.0.lock().unwrap().is_empty());
    }
}
