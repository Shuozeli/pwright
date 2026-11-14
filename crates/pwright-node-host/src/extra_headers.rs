//! Opt-in Rust host installation. Synchronous Rust preparation owns desired
//! metadata; the existing topology driver owns native writes and attachment.
//! No Node values, worker emulation or second event cursor.
mod fanout;
mod jobs;
mod native;
pub(crate) use jobs::Jobs;
mod navigation;
mod preparation;
mod session_retirement;
pub(crate) use native::Initializer;
use preparation::{DispatchPlan, PreparedUpdate};
pub(crate) use session_retirement::Observer;
#[cfg(test)]
mod tests;

use crate::{HostError, HostSession, LiveSession, lifecycle::DriverRequest};
use pwright_backend_cdp_topology::CdpContextOwnership;
use pwright_cdp::CdpConnection;
use pwright_core::{
    ObjectId, ProductionResourceLedger, ResourceClass, ResourceLease, ResourceRequirements,
    ResourceScope,
};
pub use pwright_network_model::extra_headers::HeaderRef;
use pwright_network_model::extra_headers::settings::{
    HeaderSettings, HeaderSettingsProfile, SettingsError,
};
use pwright_network_model::extra_headers::{
    HeaderError, HeaderPool, HeaderProfile, HeaderSyntaxError, PageHeaders,
};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};
use tokio::sync::oneshot;

const MAX_HEADERS: usize = 256;
const HEADER_BYTES: usize = 64 * 1024;
pub const MAX_EXTRA_HEADERS: usize = MAX_HEADERS;
pub const MAX_EXTRA_HEADER_BYTES: usize = HEADER_BYTES;
const MAX_ROUTES: usize = 80; // 16 Pages and at most 64 Frame sessions.
const ID_BYTES: usize = 256;
const MAX_REQUESTS: usize = 9; // Shared by queued, executing and draining owners.
const REQUEST_PLAN_BYTES: usize = 128 * 1024;
const REQUEST_BYTES: usize = REQUEST_PLAN_BYTES
    + jobs::FUTURE_BYTES
    + 2 * MAX_ROUTES * (fanout::FUTURE_BYTES + fanout::NODE_BYTES);
const NATIVE_BYTES: usize = 160 * 1024;
const MAX_APPLIED: usize = 2 * MAX_ROUTES;
const MAX_FAILED_PAGES: usize = 16;
// Sparse settings (32), nine prior/input + per-Page dispatch owners, up to
// 80 initializers, nine 16-Page compensation plans and a temporary proposal.
const MAX_VALUE_OWNERS: usize = 2 * MAX_FAILED_PAGES
    + MAX_REQUESTS * (MAX_FAILED_PAGES + 2)
    + MAX_ROUTES
    + MAX_REQUESTS * MAX_FAILED_PAGES
    + 1;

/// Bounded owned launch input. Native installation later transfers these
/// values into the existing resource-accounted header pool without retaining
/// a second settings implementation.
pub struct PersistentContextExtraHeaders {
    entries: Vec<(String, String)>,
}

impl PersistentContextExtraHeaders {
    pub fn new(entries: Vec<(String, String)>) -> Result<Self, HostError> {
        if entries.len() > MAX_HEADERS {
            return Err(HostError::ResourceExhausted(
                "persistent Context extra header count exceeds 256".into(),
            ));
        }
        let bytes = entries.iter().try_fold(0usize, |total, (name, value)| {
            total
                .checked_add(name.len())
                .and_then(|total| total.checked_add(value.len()))
                .ok_or_else(|| {
                    HostError::ResourceExhausted(
                        "persistent Context extra header bytes overflow".into(),
                    )
                })
        })?;
        if bytes > HEADER_BYTES {
            return Err(HostError::ResourceExhausted(
                "persistent Context extra header bytes exceed 65536".into(),
            ));
        }
        Ok(Self { entries })
    }

    fn references(&self) -> Vec<HeaderRef<'_>> {
        self.entries
            .iter()
            .map(|(name, value)| HeaderRef { name, value })
            .collect()
    }
}

enum PersistentDefaultHeaders {
    Unselected,
    Pending(PageHeaders),
    Registered(ObjectId),
}

struct PersistentDefault(Mutex<PersistentDefaultHeaders>);

impl PersistentDefault {
    fn new(headers: Option<PageHeaders>) -> Self {
        Self(Mutex::new(match headers {
            Some(headers) => PersistentDefaultHeaders::Pending(headers),
            None => PersistentDefaultHeaders::Unselected,
        }))
    }

    fn seed(
        &self,
        settings: &Mutex<HeaderSettings>,
        context: ObjectId,
        ownership: CdpContextOwnership,
    ) -> Result<(), HostError> {
        if ownership != CdpContextOwnership::Default {
            return Ok(());
        }
        let mut state = self.0.lock().map_err(HostError::backend)?;
        match &*state {
            PersistentDefaultHeaders::Unselected => Ok(()),
            PersistentDefaultHeaders::Registered(existing) if *existing == context => Ok(()),
            PersistentDefaultHeaders::Registered(_) => Err(HostError::Backend(
                "persistent headers observed multiple default Context identities".into(),
            )),
            PersistentDefaultHeaders::Pending(headers) => {
                let input: Vec<_> = headers.iter().collect();
                let settings = settings.lock().map_err(HostError::backend)?;
                settings
                    .begin_context(context, &input)
                    .map_err(settings_error)?
                    .commit()
                    .map_err(settings_error)?;
                *state = PersistentDefaultHeaders::Registered(context);
                Ok(())
            }
        }
    }
}

struct InitializationFailures(Vec<(ObjectId, HeaderSyntaxError)>);
impl InitializationFailures {
    fn new() -> Self {
        Self(Vec::with_capacity(MAX_FAILED_PAGES))
    }
    fn record(&mut self, page: ObjectId, error: HeaderSyntaxError) -> Result<(), HostError> {
        if self.0.iter().any(|(id, _)| *id == page) {
            return Ok(()); // First initialization failure stays latched.
        }
        if self.0.len() == MAX_FAILED_PAGES {
            return Err(HostError::ResourceExhausted(
                "header initialization failure capacity exhausted".into(),
            ));
        }
        self.0.push((page, error));
        Ok(())
    }
    fn get(&self, page: ObjectId) -> Option<HeaderSyntaxError> {
        self.0
            .iter()
            .find(|(id, _)| *id == page)
            .map(|(_, error)| *error)
    }
    fn retain(&mut self, mut live: impl FnMut(ObjectId) -> bool) {
        self.0.retain(|(page, _)| live(*page));
    }
}

fn value_profile() -> Result<HeaderProfile, HostError> {
    HeaderProfile::new(
        ResourceScope::extension(127, 0),
        MAX_VALUE_OWNERS,
        MAX_HEADERS,
        HEADER_BYTES,
    )
    .map_err(HostError::backend)
}
fn settings_profile() -> Result<HeaderSettingsProfile, HostError> {
    HeaderSettingsProfile::new(ResourceScope::extension(127, 1), 16, 16)
        .and_then(|profile| profile.with_concurrent_writers(MAX_REQUESTS))
        .map_err(HostError::backend)
}
fn body_requirements() -> ResourceRequirements {
    // Snapshot-cell/group queues fit inside the existing request-plan allowance.
    debug_assert!(native::plan_scratch_bytes() <= 16 * 1024);
    debug_assert!(MAX_ROUTES * session_retirement::route_bytes() <= 96 * 1024);
    // Fixed reservation, not a claim that these bytes are resident RAM. Covers
    // registry wrapper/Arc metadata; nine request bodies, borrowed header arrays,
    // bounded route copies; and 80 simultaneous initialization/application
    // scratch areas (header strings + 256 bytes/map entry + 32 KiB future/error
    // allowance). Native encoded wire/response owners have the CDP profile.
    // Allocator padding, task stacks and foreign-runtime values remain external.
    let bytes = (std::mem::size_of::<Installation>()
        + 1024
        + MAX_APPLIED * (std::mem::size_of::<String>() + ID_BYTES)
        + MAX_FAILED_PAGES * std::mem::size_of::<(ObjectId, HeaderSyntaxError)>()
        + session_retirement::registry_bytes()
        + navigation::MAX_OPERATIONS * navigation::OPERATION_BYTES
        + REQUEST_BYTES // Driver-only obsolete-session reconciliation scratch.
        + MAX_REQUESTS * REQUEST_BYTES
        + MAX_ROUTES * NATIVE_BYTES) as u64;
    ResourceRequirements::none()
        .with_class(ResourceClass::BackendStateBytes, bytes, bytes)
        .with_metadata(1, 1)
}
pub(crate) fn requirements() -> Result<ResourceRequirements, HostError> {
    Ok(value_profile()?
        .requirements()
        .map_err(HostError::backend)?
        .combine_concurrent(settings_profile()?.requirements())
        .combine_concurrent(body_requirements()))
}

pub(crate) struct Installation {
    connection: Arc<CdpConnection>,
    pool: HeaderPool,
    // Host preparation lock: publication and dispatch-plan admission are one
    // synchronous transaction. Initializers cannot observe a rejected plan.
    // Never held across native awaits. The model retains its own portable lock.
    settings: Arc<Mutex<HeaderSettings>>,
    requests: AtomicUsize,
    navigations: AtomicUsize,
    native: tokio::sync::Semaphore,
    applied: Mutex<Vec<String>>,
    watches: session_retirement::Watches,
    failed_initializations: Mutex<InitializationFailures>,
    persistent_default: PersistentDefault,
    _lease: ResourceLease,
}
impl Installation {
    pub(crate) fn new_with_default(
        connection: Arc<CdpConnection>,
        resources: &ProductionResourceLedger,
        persistent_default: Option<PersistentContextExtraHeaders>,
    ) -> Result<Arc<Self>, HostError> {
        let req = body_requirements();
        let lease = resources
            .claim(req)
            .map_err(HostError::backend)?
            .reserve(
                ResourceScope::extension(127, 2),
                ResourceClass::BackendStateBytes,
                req.class(ResourceClass::BackendStateBytes).global,
            )
            .map_err(HostError::backend)?;
        let pool = HeaderPool::new(value_profile()?, resources).map_err(HostError::backend)?;
        let persistent_default = persistent_default
            .map(|headers| {
                let input = headers.references();
                pool.page(&input).map_err(input_error)
            })
            .transpose()?;
        let settings = HeaderSettings::new(settings_profile()?, pool.clone(), resources)
            .map_err(HostError::backend)?;
        Ok(Arc::new(Self {
            connection,
            pool,
            settings: Arc::new(Mutex::new(settings)),
            requests: AtomicUsize::new(0),
            navigations: AtomicUsize::new(0),
            native: tokio::sync::Semaphore::new(MAX_ROUTES),
            applied: Mutex::new(Vec::with_capacity(MAX_APPLIED)),
            failed_initializations: Mutex::new(InitializationFailures::new()),
            watches: session_retirement::Watches::new(),
            persistent_default: PersistentDefault::new(persistent_default),
            _lease: lease,
        }))
    }

    fn request(
        self: &Arc<Self>,
        live: &LiveSession,
        target: Target,
        input: &[HeaderRef<'_>],
        reply: oneshot::Sender<Result<(), HostError>>,
    ) -> Result<Request, HostError> {
        admit(&self.requests, MAX_REQUESTS)?;
        let permit = RequestPermit(self.clone());
        let (context, routes) = routes(live, target)?;
        let settings = self.settings.lock().map_err(HostError::backend)?;
        let update = match target {
            Target::Context(_) => settings.begin_context(context, input),
            Target::Page(page) => settings.begin_page(context, page, input),
        }
        .map_err(settings_error)?;
        let plan = match DispatchPlan::capture(&settings, routes) {
            Ok(plan) => plan,
            Err(error) => {
                // No initializer/navigation reader can observe this proposal
                // while the preparation lock is held. No native work started.
                update.rollback().map_err(HostError::backend)?;
                return Err(error);
            }
        };
        Ok(Request {
            plan,
            update: PreparedUpdate::new(self.settings.clone(), update),
            reply,
            _permit: permit,
        })
    }

    pub(crate) fn creation(
        self: &Arc<Self>,
        input: &[HeaderRef<'_>],
    ) -> Result<CreationHeaders, HostError> {
        admit(&self.requests, MAX_REQUESTS)?;
        let permit = RequestPermit(self.clone());
        let input = self.pool.page(input).map_err(input_error)?;
        Ok(CreationHeaders { input, permit })
    }

    #[cfg(feature = "page-routes")]
    pub(crate) fn augment_route_request_headers(
        &self,
        context: ObjectId,
        page: ObjectId,
        headers: &mut Vec<(String, String)>,
    ) -> Result<(), HostError> {
        let page_headers = self
            .settings
            .lock()
            .map_err(HostError::backend)?
            .page_headers(context, page)
            .map_err(HostError::backend)?;
        include_navigation_referrer(headers, page_headers.as_ref())
    }

    pub(crate) fn prune(&self, live: &LiveSession) -> Result<(), HostError> {
        let topology = live.composition.topology().map_err(HostError::backend)?;
        let topology = topology.topology();
        self.failed_initializations
            .lock()
            .map_err(HostError::backend)?
            .retain(|id| {
                topology
                    .page_session_routes()
                    .any(|page| page.object_id() == id)
            });
        self.settings
            .lock()
            .map_err(HostError::backend)?
            .retain_live(
                |id| {
                    topology
                        .browser_context_routes()
                        .any(|route| route.object_id() == id)
                },
                |context, page| {
                    topology.page_session_routes().any(|route| {
                        route.object_id() == page && route.browser_context_object_id() == context
                    })
                },
            )
            .map_err(HostError::backend)
    }
}

#[cfg(feature = "page-routes")]
fn include_navigation_referrer(
    headers: &mut Vec<(String, String)>,
    page_headers: Option<&PageHeaders>,
) -> Result<(), HostError> {
    if headers
        .iter()
        .any(|(name, _)| name.eq_ignore_ascii_case("referer"))
    {
        return Ok(());
    }
    let Some(value) = page_headers
        .map(|headers| headers.navigation_referrer(None))
        .transpose()
        .map_err(HostError::backend)?
        .flatten()
    else {
        return Ok(());
    };
    if headers.len() == MAX_HEADERS {
        return Err(HostError::ResourceExhausted(
            "route header count exceeds 256".into(),
        ));
    }
    let bytes = headers.iter().try_fold(0usize, |total, (name, value)| {
        total
            .checked_add(name.len())
            .and_then(|total| total.checked_add(value.len()))
    });
    let bytes = bytes
        .and_then(|total| total.checked_add("referer".len()))
        .and_then(|total| total.checked_add(value.len()))
        .ok_or_else(|| HostError::ResourceExhausted("route header bytes overflow".into()))?;
    if bytes > HEADER_BYTES {
        return Err(HostError::ResourceExhausted(
            "route header bytes exceed 65536".into(),
        ));
    }
    headers
        .try_reserve_exact(1)
        .map_err(|error| HostError::ResourceExhausted(error.to_string()))?;
    headers.push(("referer".to_owned(), value.to_owned()));
    Ok(())
}

fn input_error(error: HeaderError) -> HostError {
    match error {
        HeaderError::CountLimit
        | HeaderError::ByteLimit
        | HeaderError::OwnerLimit
        | HeaderError::Budget(_) => HostError::ResourceExhausted(error.to_string()),
        HeaderError::InvalidLimits | HeaderError::ReferrerConflict | HeaderError::Plan(_) => {
            HostError::backend(error)
        }
    }
}

fn settings_error(error: SettingsError) -> HostError {
    match error {
        SettingsError::Headers(error) => input_error(error),
        SettingsError::Busy | SettingsError::Capacity => {
            HostError::ResourceExhausted(error.to_string())
        }
        error => HostError::backend(error),
    }
}

fn syntax_error(error: HeaderSyntaxError) -> HostError {
    let message = match error {
        HeaderSyntaxError::NonAsciiName => "Invalid parameters",
        HeaderSyntaxError::InvalidName => "Invalid header name",
        HeaderSyntaxError::InvalidValue => "Invalid header value",
    };
    HostError::Backend(format!(
        "Protocol error (Network.setExtraHTTPHeaders): {message}"
    ))
}

/// Return an exact created Page's latched failure for lifecycle compensation.
/// The caller closes that Page and rejects creation, preserving its Context.
pub(crate) fn creation_failure(
    state: &Mutex<Option<LiveSession>>,
    reply: &crate::lifecycle::DriverReply,
) -> Result<Option<(pwright_core::CallId, ObjectId, HostError)>, HostError> {
    if let crate::lifecycle::DriverReply::NewPage {
        call_id, result, ..
    } = reply
        && let Ok(created) = result
    {
        let id = created.id.parse::<u64>().map_err(HostError::backend)?;
        let state = state.lock().map_err(HostError::backend)?;
        let live = state.as_ref().ok_or(HostError::TargetClosed)?;
        let failures = live
            .extra_headers
            .failed_initializations
            .lock()
            .map_err(HostError::backend)?;
        if let Some(error) = failures.get(ObjectId::new(id)) {
            return Ok(Some((*call_id, ObjectId::new(id), syntax_error(error))));
        }
    }
    Ok(None)
}

/// A replaced topology route does not prove that the previous CDP session was
/// detached. Clear only header side effects installed by this connection before
/// forgetting such a session; otherwise clearing the current session can reveal
/// the old session's headers in Chromium. Other capability leases remain intact.
pub(crate) async fn reconcile_native(state: &Mutex<Option<LiveSession>>) -> Result<(), HostError> {
    let (installation, stale) = {
        let state = state.lock().map_err(HostError::backend)?;
        let live = state.as_ref().ok_or(HostError::TargetClosed)?;
        let installation = &live.extra_headers;
        let topology = live.composition.topology().map_err(HostError::backend)?;
        let applied = installation.applied.lock().map_err(HostError::backend)?;
        // Most native events do not replace a route. Keep that hot path free
        // of a scratch allocation, including when no headers were ever set.
        let mut stale = Vec::new();
        for session in applied.iter() {
            if !topology
                .topology()
                .target_session_routes()
                .any(|route| route.session_id() == session)
            {
                if stale.is_empty() {
                    stale.reserve_exact(MAX_APPLIED);
                }
                stale.push(session.clone());
            }
        }
        (installation.clone(), stale)
    };
    for session in stale {
        installation.clear_obsolete(&session).await?;
    }
    Ok(())
}

fn admit(counter: &AtomicUsize, limit: usize) -> Result<(), HostError> {
    counter
        .fetch_update(Ordering::AcqRel, Ordering::Acquire, |active| {
            (active < limit).then_some(active + 1)
        })
        .map(|_| ())
        .map_err(|_| {
            HostError::ResourceExhausted("extra header operation capacity exhausted".into())
        })
}
struct RequestPermit(Arc<Installation>);
impl Drop for RequestPermit {
    fn drop(&mut self) {
        self.0.requests.fetch_sub(1, Ordering::AcqRel);
    }
}
#[derive(Clone, Copy)]
enum Target {
    Context(ObjectId),
    Page(ObjectId),
}
pub(crate) struct Request {
    plan: DispatchPlan,
    update: PreparedUpdate,
    pub(crate) reply: oneshot::Sender<Result<(), HostError>>,
    _permit: RequestPermit,
}

/// Admitted before native Context creation; no language handles cross the queue.
/// The driver installs desired settings before publishing the Context descriptor.
pub(crate) struct CreationHeaders {
    input: PageHeaders,
    permit: RequestPermit,
}
impl CreationHeaders {
    pub(crate) fn commit(self, context: ObjectId) -> Result<(), HostError> {
        let input: Vec<_> = self.input.iter().collect();
        self.permit
            .0
            .settings
            .lock()
            .map_err(HostError::backend)?
            .begin_context(context, &input)
            .map_err(HostError::backend)?
            .commit()
            .map_err(HostError::backend)
    }
}
#[derive(Clone)]
struct Route {
    context: ObjectId,
    page: ObjectId,
    watch: Arc<session_retirement::Watch>,
}

fn routes(live: &LiveSession, target: Target) -> Result<(ObjectId, Vec<Route>), HostError> {
    let topology = live.composition.topology().map_err(HostError::backend)?;
    let topology = topology.topology();
    let context = match target {
        Target::Context(id) => topology
            .browser_context_routes()
            .find(|route| route.object_id() == id)
            .map(|route| route.object_id()),
        Target::Page(id) => topology
            .page_session_routes()
            .find(|route| route.object_id() == id)
            .map(|route| route.browser_context_object_id()),
    }
    .ok_or(HostError::TargetClosed)?;
    let mut routes = Vec::with_capacity(MAX_ROUTES);
    for native in topology.target_session_routes() {
        let page = topology
            .page_session_routes()
            .find(|page| page.target_id() == native.page_target_id())
            .ok_or_else(|| HostError::Backend("header session has no owning Page".into()))?;
        let selected = match target {
            Target::Context(id) => page.browser_context_object_id() == id,
            Target::Page(id) => page.object_id() == id,
        };
        if !selected {
            continue;
        }
        if routes.len() == MAX_ROUTES || native.session_id().len() > ID_BYTES {
            return Err(HostError::ResourceExhausted(
                "extra header native route bound exceeded".into(),
            ));
        }
        routes.push(Route {
            context,
            page: page.object_id(),
            watch: live.extra_headers.watches.insert(
                native.session_id(),
                page.session_id(),
                native.target_id(),
            )?,
        });
    }
    Ok((context, routes))
}

/// Caller-polled native work. Public failure does not abandon primary siblings
/// or compensation; this request keeps its permit until both have drained.
async fn execute(mut request: Request) {
    if request.reply.is_closed() {
        return;
    }
    let installation = &request._permit.0;
    let mut primary = match installation.fanout(&request.plan, false) {
        Ok(primary) => primary,
        Err(error) => {
            let _ = request.reply.send(Err(error));
            return;
        }
    };
    match primary.first_result().await {
        Ok(()) => {
            let _ = request.reply.send(request.update.commit());
        }
        Err(error) => {
            let plan = request.update.restore_plan(&request.plan);
            let mut compensation = plan
                .as_ref()
                .ok()
                .and_then(|plan| installation.fanout(plan, true).ok());
            reject_and_drain(
                &mut request.update,
                &mut primary,
                compensation.as_mut(),
                request.reply,
                error,
            )
            .await;
            // Unconfirmed compensation leaves the update unresolved. Drop marks
            // uncertainty without repeating its already published restoration.
        }
    }
}

async fn reject_and_drain(
    update: &mut PreparedUpdate,
    primary: &mut fanout::Fanout<'_>,
    compensation: Option<&mut fanout::Fanout<'_>>,
    reply: oneshot::Sender<Result<(), HostError>>,
    error: HostError,
) {
    if let Some(compensation) = compensation {
        compensation.start().await;
        let _ = reply.send(Err(error));
        let (_, restored) = tokio::join!(primary.drain(), compensation.settle());
        if restored.is_ok() {
            let _ = update.rollback();
        }
    } else {
        let _ = reply.send(Err(error));
        primary.drain().await;
    }
}

/// Portable target for synchronous Rust preparation at a language-call boundary.
pub enum NativeHeaderTarget<'a> {
    Context(&'a str),
    Page(&'a str),
}

struct Completion {
    receiver: oneshot::Receiver<Result<(), HostError>>,
    _operation: tokio_util::task::task_tracker::TaskTrackerToken,
}

impl HostSession {
    /// Publish desired headers and reserve a frozen native dispatch before this
    /// method returns. Only native completion is asynchronous; no input borrow
    /// or language handle escapes into the returned future.
    pub fn start_set_extra_http_headers_native(
        self: &Arc<Self>,
        target: NativeHeaderTarget<'_>,
        input: &[HeaderRef<'_>],
    ) -> impl Future<Output = Result<(), HostError>> + Send + 'static + use<> {
        let prepared = self.prepare_headers_native(target, input);
        let host = self.clone();
        async move {
            let Completion {
                receiver,
                _operation,
            } = prepared?;
            host.await_driver_reply(receiver).await
        }
    }

    /// Provisional native port: applies to current Page/OOPIF sessions and future
    /// attachments. No Node conversion, worker fanout or public failure-timing
    /// compatibility is implied. Caller-owned borrowed input is not retained.
    pub async fn set_context_extra_http_headers_native(
        &self,
        context: &str,
        input: &[HeaderRef<'_>],
    ) -> Result<(), HostError> {
        let Completion {
            receiver,
            _operation,
        } = self.prepare_headers_native(NativeHeaderTarget::Context(context), input)?;
        self.await_driver_reply(receiver).await
    }

    /// Page-only override; an empty list reveals the Context's current headers.
    pub async fn set_page_extra_http_headers_native(
        &self,
        page: &str,
        input: &[HeaderRef<'_>],
    ) -> Result<(), HostError> {
        let Completion {
            receiver,
            _operation,
        } = self.prepare_headers_native(NativeHeaderTarget::Page(page), input)?;
        self.await_driver_reply(receiver).await
    }

    fn prepare_headers_native(
        &self,
        target: NativeHeaderTarget<'_>,
        input: &[HeaderRef<'_>],
    ) -> Result<Completion, HostError> {
        let id = match target {
            NativeHeaderTarget::Context(id) | NativeHeaderTarget::Page(id) => id,
        };
        let (id, _, _operation) = self.admit_operation(id)?;
        // Reserve queue capacity before publishing anything. A full queue is
        // a pure admission failure, not a published-and-abandoned update.
        let slot = self
            .topology_barriers
            .try_reserve()
            .map_err(|error| match error {
                tokio::sync::mpsc::error::TrySendError::Full(_) => HostError::ResourceExhausted(
                    "topology/lifecycle request capacity exceeded (8)".into(),
                ),
                tokio::sync::mpsc::error::TrySendError::Closed(_) => HostError::TargetClosed,
            })?;
        let (reply, completion) = oneshot::channel();
        let request = {
            let state = self.state.lock().map_err(HostError::backend)?;
            let live = state.as_ref().ok_or(HostError::TargetClosed)?;
            let target = match target {
                NativeHeaderTarget::Context(_) => Target::Context(id),
                NativeHeaderTarget::Page(_) => Target::Page(id),
            };
            live.extra_headers.request(live, target, input, reply)?
        };
        slot.send(DriverRequest::ExtraHeaders(request));
        Ok(Completion {
            receiver: completion,
            _operation,
        })
    }
}
