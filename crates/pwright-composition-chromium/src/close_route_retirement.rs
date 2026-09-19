//! Borrowed, deletion-only Page/Frame routing commit. No native-death authority,
//! topology/runtime mutation, route factories, or cloned routing tables.
use crate::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CloseRouteRetirementError {
    EpochMismatch,
    NotDeletionOnly,
    StatePoisoned,
    Network(ChromiumNetworkSubscriptionStateError),
}
impl Display for CloseRouteRetirementError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EpochMismatch => f.write_str("close route epoch differs from current routes"),
            Self::NotDeletionOnly => {
                f.write_str("close route plan is not an exact deletion-only view")
            }
            Self::StatePoisoned => f.write_str("close route state is poisoned"),
            Self::Network(error) => Display::fmt(error, f),
        }
    }
}
impl Error for CloseRouteRetirementError {}
type Result<T> = std::result::Result<T, CloseRouteRetirementError>;

/// Holds only borrowed locks/IDs. Drop leaves routes, navigation snapshots and
/// subscription state unchanged. Native proof and commit scratch belong to the
/// selecting coordinator, not this local routing primitive.
#[must_use = "commit route retirement or drop to preserve the current view"]
pub struct PreparedCloseRouteRetirement<'a> {
    current: MutexGuard<'a, CdpServiceRoutes>,
    network: Option<MutexGuard<'a, ChromiumNetworkSubscriptionState>>,
    disposed: &'a [ObjectId],
    readiness: Option<&'a Arc<ExecutionWorldReadiness>>,
}

/// Explicit post-commit wake port. Invoke only after releasing ALL surrounding
/// topology/runtime/handle locks; user-supplied wakers may reenter the host.
#[must_use = "notify readiness waiters after the complete local commit"]
pub struct CloseRouteRetirementWake<'a> {
    readiness: Option<&'a Arc<ExecutionWorldReadiness>>,
}
impl CloseRouteRetirementWake<'_> {
    pub fn notify(self) {
        if let Some(readiness) = self.readiness {
            readiness.committed();
        }
    }
}

impl CdpPageServices {
    /// Validate the exact survivor view against a prepared topology candidate.
    /// This is not a native receipt or an independently authorized lifecycle
    /// transaction. The caller must retain its topology/runtime gates and exact
    /// native proof across this SHORT synchronous prepare/commit phase only.
    ///
    /// The candidate and runtime child-first disposal IDs are borrowed. No
    /// allocation, service construction, native command or subscription is made.
    pub fn prepare_close_route_retirement<'a>(
        &'a self,
        candidate: &pwright_backend_cdp::CdpTopology,
        disposed: &'a [ObjectId],
    ) -> Result<PreparedCloseRouteRetirement<'a>> {
        let current = self
            .routes
            .lock()
            .map_err(|_| CloseRouteRetirementError::StatePoisoned)?;
        if current.epoch != Some(candidate.epoch()) {
            return Err(CloseRouteRetirementError::EpochMismatch);
        }
        validate_survivors(self, &current, candidate, disposed)?;
        // Same lock order as ordinary synchronization: routes, then network
        // subscriptions. Retain the network guard so commit cannot fail or
        // silently ignore a poisoned lock after topology retirement.
        let network = self
            .network_subscriptions
            .as_ref()
            .map(|subscriptions| {
                subscriptions
                    .shared
                    .state
                    .lock()
                    .map_err(|_| CloseRouteRetirementError::StatePoisoned)
            })
            .transpose()?;
        if let Some(network) = &network {
            validate_network(network, &current, disposed)?;
        }
        Ok(PreparedCloseRouteRetirement {
            current,
            network,
            disposed,
            readiness: self.world_readiness.as_ref(),
        })
    }
}

impl<'a> PreparedCloseRouteRetirement<'a> {
    /// Record an independently validated native fact, not local route absence.
    #[cfg(feature = "native-close-retirement")]
    pub(crate) fn observe_native_destruction(
        &self,
        event: native_world_retirement::Destruction<'_>,
    ) -> std::result::Result<(), CdpPageRouteError> {
        if let Some(registry) = &self.current.native_world_retirement {
            registry.mark(self.current.epoch.expect("validated close epoch"), event)?;
        }
        Ok(())
    }

    /// Infallible, allocation-free deletion. Keeps all surviving Arc identities
    /// and the native-world receipt registry. Does NOT mark any world destroyed.
    pub fn commit(mut self) -> CloseRouteRetirementWake<'a> {
        let disposed = self.disposed;
        let keep = |id: &ObjectId| !disposed.contains(id);
        if let Some(table) = &mut self.current.frame_navigation {
            table.retire(disposed);
        }
        self.current.browser_contexts.retain(keep);
        self.current.pages.retain(|id, _| keep(id));
        self.current.frame_evaluations.retain(|id, _| keep(id));
        self.current
            .frame_handle_evaluations
            .retain(|id, _| keep(id));
        self.current.frame_contexts.retain(|id, _| keep(id));
        self.current
            .locator_screenshot_contexts
            .retain(|id, _| keep(id));
        self.current
            .readiness_frames
            .retain(|(frame, _)| keep(frame));
        if let Some(network) = &mut self.network {
            network.lanes.retain(|page, lane| {
                self.current
                    .pages
                    .get(page)
                    .is_some_and(|route| route.session_id == lane.session_id)
            });
            network
                .contexts
                .retain(|id, _| self.current.browser_contexts.contains(id));
        }
        drop(self.network);
        drop(self.current);
        CloseRouteRetirementWake {
            readiness: self.readiness,
        }
    }
}

fn matches_entries<V, D>(
    current: &BTreeMap<ObjectId, Arc<V>>,
    desired: impl Iterator<Item = (ObjectId, D)>,
    disposed: &[ObjectId],
    same: impl Fn(&V, D) -> bool,
) -> bool {
    let mut count = 0;
    for (id, desired) in desired {
        if disposed.contains(&id) || !current.get(&id).is_some_and(|entry| same(entry, desired)) {
            return false;
        }
        count += 1;
    }
    count == current.keys().filter(|id| !disposed.contains(id)).count()
}
fn worlds(
    topology: &pwright_backend_cdp::CdpTopology,
    utility: bool,
) -> impl Iterator<Item = pwright_backend_cdp::CdpFrameExecutionRoute<'_>> {
    topology
        .frame_execution_routes()
        .filter(move |_| !utility)
        .chain(
            topology
                .frame_utility_execution_routes()
                .filter(move |_| utility),
        )
}
fn validate_survivors(
    services: &CdpPageServices,
    current: &CdpServiceRoutes,
    candidate: &pwright_backend_cdp::CdpTopology,
    disposed: &[ObjectId],
) -> Result<()> {
    let invalid = || CloseRouteRetirementError::NotDeletionOnly;
    let contexts = candidate.browser_context_routes();
    let mut count = 0;
    for context in contexts {
        if disposed.contains(&context.object_id())
            || !current.browser_contexts.contains(&context.object_id())
        {
            return Err(invalid());
        }
        count += 1;
    }
    if count
        != current
            .browser_contexts
            .iter()
            .filter(|id| !disposed.contains(id))
            .count()
    {
        return Err(invalid());
    }
    if !matches_entries(
        &current.pages,
        candidate
            .page_session_routes()
            .map(|route| (route.object_id(), route)),
        disposed,
        |entry, route| {
            entry.target_id == route.target_id()
                && entry.session_id == route.session_id()
                && entry.browser_context_object_id == route.browser_context_object_id()
                && Some(entry.main_frame_object_id)
                    == candidate.main_frame_object_id(route.target_id())
                && entry.crashed
                    == matches!(
                        candidate.page_state(route.target_id()),
                        Some(CdpPageState::Crashed { .. })
                    )
        },
    ) {
        return Err(invalid());
    }
    for (entries, selected) in [
        (&current.frame_evaluations, services.evaluation.is_some()),
        (
            &current.frame_handle_evaluations,
            services.handle_evaluation.is_some(),
        ),
    ] {
        if !matches_entries(
            entries,
            candidate
                .frame_execution_routes()
                .filter(|_| selected)
                .map(|r| (r.object_id(), r)),
            disposed,
            |entry, route| {
                entry.target_id == route.target_id()
                    && entry.session_id == route.session_id()
                    && entry.execution_context_id == route.execution_context_id()
                    && entry.unique_context_id.as_ref() == route.unique_context_id()
                    && Some(entry.page_object_id)
                        == candidate.page_object_id_for_frame(route.object_id())
            },
        ) {
            return Err(invalid());
        }
    }
    let context_worlds = worlds(
        candidate,
        services.dom_content.is_some() || services.locator_uses_utility_world,
    )
    .filter(|_| services.locator.is_some() || services.dom_content.is_some());
    if !matches_entries(
        &current.frame_contexts,
        context_worlds.map(|r| (r.object_id(), r)),
        disposed,
        |entry, route| {
            entry.target_id == route.target_id()
                && entry.session_id == route.session_id()
                && entry.unique_context_id.as_ref() == route.unique_context_id()
                && entry.locator.is_some() == services.locator.is_some()
                && entry.dom_content.is_some() == services.dom_content.is_some()
                && entry.page_crashed == crashed(candidate, route.object_id())
        },
    ) {
        return Err(invalid());
    }
    let screenshot_worlds = worlds(candidate, services.locator_screenshot_uses_utility_world)
        .filter(|_| services.locator_screenshot_locator.is_some());
    if !matches_entries(
        &current.locator_screenshot_contexts,
        screenshot_worlds.map(|r| (r.object_id(), r)),
        disposed,
        |entry, route| {
            entry.target_id == route.target_id()
                && entry.session_id == route.session_id()
                && entry.unique_context_id.as_ref() == route.unique_context_id()
                && Some(entry.page_object_id)
                    == candidate.page_object_id_for_frame(route.object_id())
                && Some(entry.frame_id.as_ref()) == candidate.frame_native_id(route.object_id())
                && entry.parent_frame_object_id
                    == candidate.parent_frame_object_id(route.object_id())
                && entry.page_crashed == crashed(candidate, route.object_id())
        },
    ) {
        return Err(invalid());
    }
    if let Some(table) = &current.frame_navigation
        && !table.matches_retirement(candidate, disposed)
    {
        return Err(invalid());
    }
    if services.world_readiness.is_some() {
        let mut count = 0;
        for pair in candidate.frame_page_routes() {
            if disposed.contains(&pair.0) || !current.readiness_frames.contains(&pair) {
                return Err(invalid());
            }
            count += 1;
        }
        if count
            != current
                .readiness_frames
                .iter()
                .filter(|(frame, _)| !disposed.contains(frame))
                .count()
        {
            return Err(invalid());
        }
    }
    Ok(())
}
fn crashed(topology: &pwright_backend_cdp::CdpTopology, frame: ObjectId) -> bool {
    topology
        .page_object_id_for_frame(frame)
        .is_some_and(|page| {
            topology
                .page_session_routes()
                .find(|route| route.object_id() == page)
                .is_some_and(|route| {
                    matches!(
                        topology.page_state(route.target_id()),
                        Some(CdpPageState::Crashed { .. })
                    )
                })
        })
}
fn validate_network(
    network: &ChromiumNetworkSubscriptionState,
    current: &CdpServiceRoutes,
    disposed: &[ObjectId],
) -> Result<()> {
    if let Some(pending) = &network.pending_context_transition {
        return Err(CloseRouteRetirementError::Network(
            ChromiumNetworkSubscriptionStateError::BrowserContextTransitionInProgress {
                browser_context: pending.browser_context,
            },
        ));
    }
    for context in &current.browser_contexts {
        if !disposed.contains(context) && !network.contexts.contains_key(context) {
            return Err(CloseRouteRetirementError::Network(
                ChromiumNetworkSubscriptionStateError::BrowserContextUnavailable {
                    browser_context: *context,
                },
            ));
        }
    }
    for (page, route) in &current.pages {
        if disposed.contains(page) {
            continue;
        }
        let active = network
            .contexts
            .get(&route.browser_context_object_id)
            .is_some_and(|context| context.interests != 0);
        if active
            && !network
                .lanes
                .get(page)
                .is_some_and(|lane| lane.session_id == route.session_id)
        {
            return Err(CloseRouteRetirementError::Network(
                ChromiumNetworkSubscriptionStateError::InheritedPageLaneUnavailable {
                    browser_context: route.browser_context_object_id,
                    page: *page,
                },
            ));
        }
    }
    Ok(())
}
