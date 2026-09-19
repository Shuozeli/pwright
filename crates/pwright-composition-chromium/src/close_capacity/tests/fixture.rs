//! Real topology/runtime reservations with a bounded test publication observer.
//! No CDP, browser or production Node publication source is simulated here.
use super::*;

pub const EPOCH: ConnectionEpoch = ConnectionEpoch::new(1);
pub const PAGE: ChromiumCloseTarget = ChromiumCloseTarget::Page(ObjectId::new(3));
pub const CONTEXT: ChromiumCloseTarget = ChromiumCloseTarget::BrowserContext(ObjectId::new(2));
const OBSERVER_SCOPE: ResourceScope = ResourceScope::extension(136, 4);
const OBSERVER_BYTES: u64 = 1024;

pub struct Observer(pub Arc<Publication>);
pub struct Publication {
    limit: usize,
    pub pending: AtomicUsize,
    pub attempts: AtomicUsize,
    pub prepared: AtomicUsize,
    pub committed: AtomicUsize,
    pub reject_prepare: AtomicBool,
    ledger: ResourceLedger,
    _lease: ResourceLease,
}
fn error(message: &'static str) -> OperationError<()> {
    OperationError::new(FailureKind::ResourceExhausted, message, None)
}
impl Observer {
    pub(crate) fn requirements(limit: usize) -> ResourceRequirements {
        let bytes = (limit as u64 + 1) * OBSERVER_BYTES;
        ResourceRequirements::none()
            .with_class(ResourceClass::BackendStateBytes, bytes, bytes)
            .with_metadata(1, limit + 1)
    }
    pub(crate) fn new(limit: usize, resources: &ProductionResourceLedger) -> Self {
        let ledger = resources.claim(Self::requirements(limit)).unwrap();
        let lease = ledger
            .reserve(
                OBSERVER_SCOPE,
                ResourceClass::BackendStateBytes,
                OBSERVER_BYTES,
            )
            .unwrap();
        Self(Arc::new(Publication {
            limit,
            pending: AtomicUsize::new(0),
            attempts: AtomicUsize::new(0),
            prepared: AtomicUsize::new(0),
            committed: AtomicUsize::new(0),
            reject_prepare: AtomicBool::new(false),
            ledger,
            _lease: lease,
        }))
    }
}
struct Capacity {
    owner: Arc<Publication>,
    target: ChromiumCloseTarget,
    _lease: Option<ResourceLease>,
}
impl Drop for Capacity {
    fn drop(&mut self) {
        self._lease.take();
        self.owner.pending.fetch_sub(1, Ordering::AcqRel);
    }
}
impl ChromiumCloseObserver for Observer {
    fn prepare(
        &self,
        _: ChromiumClosePlan<'_>,
    ) -> Result<Box<dyn ChromiumCloseReservation>, OperationError<()>> {
        panic!("waiting admission must not prepare a frozen publication view");
    }
    fn reserve_capacity(
        &self,
        target: ChromiumCloseTarget,
    ) -> Result<Box<dyn ChromiumCloseCapacity>, OperationError<()>> {
        self.0.attempts.fetch_add(1, Ordering::AcqRel);
        self.0
            .pending
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |n| {
                (n < self.0.limit).then_some(n + 1)
            })
            .map_err(|_| error("test publication capacity exhausted"))?;
        let mut capacity = Capacity {
            owner: self.0.clone(),
            target,
            _lease: None,
        };
        capacity._lease = Some(
            self.0
                .ledger
                .reserve(
                    OBSERVER_SCOPE,
                    ResourceClass::BackendStateBytes,
                    OBSERVER_BYTES,
                )
                .map_err(|_| error("test publication byte admission failed"))?,
        );
        Ok(Box::new(capacity))
    }
}
impl ChromiumCloseCapacity for Capacity {
    fn prepare(
        self: Box<Self>,
        plan: ChromiumClosePlan<'_>,
    ) -> Result<Box<dyn ChromiumCloseReservation>, OperationError<()>> {
        if self.owner.reject_prepare.load(Ordering::Acquire) || self.target != plan.target() {
            return Err(error("test publication preparation rejected"));
        }
        self.owner.prepared.fetch_add(1, Ordering::AcqRel);
        Ok(Box::new(Prepared {
            capacity: self,
            effects: plan.effects().len(),
        }))
    }
}
struct Prepared {
    capacity: Box<Capacity>,
    effects: usize,
}
impl ChromiumCloseReservation for Prepared {
    fn committed(self: Box<Self>, effects: &[CdpTopologyEffect]) -> Result<(), OperationError<()>> {
        assert_eq!(effects.len(), self.effects);
        self.capacity.owner.committed.fetch_add(1, Ordering::AcqRel);
        Ok(())
    }
}

pub struct Fixture {
    pub topology: ProductionCdpTopology,
    pub topology_pool: TransitionReservations,
    pub runtime: ControlledDispatcher<(), (), NoDeadlineScheduler>,
    pub runtime_pool: DisposalReservations,
    pub observer: Observer,
    pub resources: ProductionResourceLedger,
}
impl Fixture {
    pub fn new(pending: usize) -> Self {
        let topology = CdpTopologyProductionProfile::new(
            CdpTopologyLimits::new(16, 4, 4, 8, 8, 32),
            ResourceScope::extension(136, 1),
        )
        .unwrap();
        let transitions = TransitionReservationProfile::new(
            topology,
            ResourceScope::extension(136, 2),
            NonZeroUsize::new(pending).unwrap(),
        )
        .unwrap();
        let runtime_limits = RuntimeLimits::new(16, 2).with_control_messages(64);
        let disposal = DisposalReservationProfile::new(
            NonZeroUsize::new(16).unwrap(),
            NonZeroUsize::new(pending).unwrap(),
            ResourceScope::extension(136, 3),
        )
        .unwrap()
        .with_deferred_commit(NonZeroUsize::new(2).unwrap())
        .unwrap();
        let outbox = ControlOutboxStorageProfile::new(
            NonZeroUsize::new(64).unwrap(),
            ResourceScope::extension(136, 5),
        )
        .unwrap();
        let plan = topology
            .requirements()
            .combine_concurrent(transitions.requirements())
            .combine_concurrent(runtime_limits.resource_requirements())
            .combine_concurrent(disposal.requirements())
            .combine_concurrent(outbox.requirements())
            .combine_concurrent(Observer::requirements(pending));
        let mut limits = ResourceLimits::deny_all(plan.scopes, plan.leases);
        for class in ResourceClass::ALL {
            let amount = plan.class(class);
            limits = limits.with_class(class, amount.global, amount.per_scope);
        }
        let resources = ProductionResourceLedger::new(limits, plan).unwrap();
        let mut topology =
            ProductionCdpTopology::new(ObjectId::new(1), EPOCH, topology, &resources).unwrap();
        topology
            .register_context(
                EPOCH,
                "context-a",
                ObjectId::new(2),
                CdpContextOwnership::Owned,
            )
            .unwrap();
        let topology_pool = transitions.bind(&topology, &resources).unwrap();
        let runtime = ControlledDispatcher::new((), (), NoDeadlineScheduler, runtime_limits)
            .with_production_resources(&resources)
            .unwrap();
        outbox
            .install(&runtime.lifecycle_handle(), &resources)
            .unwrap();
        for (id, parent) in [(1, None), (2, Some(1))] {
            runtime
                .register_object(
                    ObjectId::new(id),
                    parent.map(ObjectId::new),
                    ObjectKindTag::new(1),
                )
                .unwrap();
        }
        let runtime_pool = disposal
            .bind(&runtime.lifecycle_handle(), &resources)
            .unwrap();
        let observer = Observer::new(pending, &resources);
        let mut fixture = Self {
            topology,
            topology_pool,
            runtime,
            runtime_pool,
            observer,
            resources,
        };
        fixture.add_page("target-a", "session-a", 3, 4);
        fixture.drain();
        fixture
    }
    pub fn add_page(&mut self, target: &str, session: &str, page: u64, frame: u64) {
        self.topology
            .attach_page(
                EPOCH,
                CdpPageTarget::new(target, session, "context-a"),
                CdpPageObjectIds::new(ObjectId::new(page), ObjectId::new(frame)),
            )
            .unwrap();
        for (id, parent) in [(page, 2), (frame, page)] {
            self.runtime
                .register_object(
                    ObjectId::new(id),
                    Some(ObjectId::new(parent)),
                    ObjectKindTag::new(1),
                )
                .unwrap();
        }
    }
    pub fn drain(&self) {
        while self.runtime.pop_control_message().unwrap().is_some() {}
    }
    pub fn reserve(
        &self,
        target: ChromiumCloseTarget,
    ) -> Result<ChromiumCloseCapacityBundle, ChromiumCloseCapacityBundleError> {
        ChromiumCloseCapacityBundle::reserve(
            target,
            &self.topology,
            &self.topology_pool,
            &self.runtime_pool,
            &self.observer,
        )
    }
    pub fn pending(&self) -> (usize, usize, usize) {
        (
            self.topology_pool.pending(),
            self.runtime_pool.pending(),
            self.observer.0.pending.load(Ordering::Acquire),
        )
    }
    pub fn current(&self) -> (u64, u64, usize) {
        let m = self.resources.metrics().unwrap();
        (
            m.class(ResourceClass::BackendStateBytes).current,
            m.class(ResourceClass::EventItem).current,
            m.active_leases,
        )
    }
    pub fn commit(&mut self, bundle: ChromiumCloseCapacityBundle) -> Vec<u64> {
        // This fixture composes local APIs only. Production must first validate
        // sealed native evidence and hold the exact outer topology/domain locks.
        let parts = bundle.into_parts();
        let object = match parts.target {
            ChromiumCloseTarget::Page(id) | ChromiumCloseTarget::BrowserContext(id) => id,
        };
        let runtime = parts
            .runtime
            .prepare(&self.runtime.lifecycle_handle(), object)
            .unwrap();
        let topology = parts
            .topology
            .prepare(&mut self.topology, |next| match parts.target {
                ChromiumCloseTarget::Page(_) => {
                    next.detach_target(EPOCH, "session-a", Some("target-a"))
                }
                ChromiumCloseTarget::BrowserContext(_) => next.close_context(EPOCH, "context-a"),
            })
            .unwrap();
        let publication = parts
            .publication
            .prepare(ChromiumClosePlan::new(
                parts.target,
                topology.output().effects(),
            ))
            .unwrap();
        let (update, mut retired) = runtime
            .commit_deferred_with_publication(None, || topology.commit(&mut self.topology))
            .unwrap()
            .unwrap();
        publication.committed(update.effects()).unwrap();
        retired.notify();
        retired
            .removed()
            .iter()
            .map(|record| record.id().get())
            .collect()
    }
    pub fn finish(self) {
        assert_eq!(self.pending(), (0, 0, 0));
        drop((
            self.observer,
            self.runtime_pool,
            self.runtime,
            self.topology_pool,
            self.topology,
        ));
        let metrics = self.resources.metrics().unwrap();
        assert_eq!(metrics.active_leases, 0);
        for class in ResourceClass::ALL {
            assert_eq!(metrics.class(class).current, 0);
        }
    }
}
