use std::{future::Future, pin::Pin, sync::Arc};

use pwright_composition_chromium::{
    CdpPageRouteError, ChromiumFrameWorldAccess, ChromiumFrameWorldSnapshot, FrameWorldAccessError,
};
use pwright_composition_chromium_locator_evaluation::{
    MainWorldRoute,
    preparation::{PairedWorld, PairedWorldSource},
};
use pwright_composition_chromium_locator_resolution::{
    ObjectGroupDestructionProof, ResolutionRoute, ResolutionWaitError, ResolutionWaitStop,
};
use pwright_core::ObjectId;

pub(super) struct Pair(ChromiumFrameWorldSnapshot);
struct Destruction(pwright_composition_chromium::ChromiumNativeWorldLifetime);
impl ObjectGroupDestructionProof for Destruction {
    fn all_group_worlds_destroyed(&self) -> bool {
        self.0.both_worlds_destroyed()
    }
    fn all_group_objects_retired(&self) -> bool {
        self.0.native_objects_retired()
    }
    fn poll_release_ready(&self, cx: &mut std::task::Context<'_>) -> std::task::Poll<()> {
        self.0.poll_session_release_ready(cx)
    }
}

struct Retirement {
    lifetime: pwright_composition_chromium::ChromiumNativeWorldLifetime,
    client: Arc<dyn pwright_cdp::CdpClient>,
    world: Arc<str>,
}
// One boxed receipt projection and one bounded copy of the native unique ID,
// including Arc/allocator allowance. Charged by host admission before creation.
pub(super) const RETIREMENT_BYTES: usize = std::mem::size_of::<Retirement>() + 1024 + 64;
impl pwright_composition_chromium_locator_evaluation::NativeEvaluationRetirement for Retirement {
    fn poll_retired(
        &mut self,
        client: &Arc<dyn pwright_cdp::CdpClient>,
        world: &str,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<()> {
        if !Arc::ptr_eq(client, &self.client) || world != self.world.as_ref() {
            return std::task::Poll::Pending;
        }
        self.lifetime.poll_session_retired(cx)
    }
}
fn route_error(error: FrameWorldAccessError) -> ResolutionWaitError {
    match error {
        FrameWorldAccessError::Topology(CdpPageRouteError::WorldTargetClosed { .. }) => {
            ResolutionWaitStop::TargetClosed.into()
        }
        FrameWorldAccessError::Topology(CdpPageRouteError::PageCrashed { .. }) => {
            ResolutionWaitStop::BrowserCrashed.into()
        }
        other => ResolutionWaitError::Route(Box::new(other)),
    }
}
impl PairedWorld for Pair {
    fn native_evaluation_retirement(
        &self,
    ) -> Option<Box<dyn pwright_composition_chromium_locator_evaluation::NativeEvaluationRetirement>>
    {
        self.0.native_lifetime().map(|lifetime| {
            Box::new(Retirement {
                lifetime,
                client: self.0.client(),
                world: self.0.main_unique_id().into(),
            })
                as Box<
                    dyn pwright_composition_chromium_locator_evaluation::NativeEvaluationRetirement,
                >
        })
    }
    fn utility_route(&self) -> ResolutionRoute {
        ResolutionRoute {
            client: self.0.client(),
            utility_world: self.0.utility_unique_id().into(),
            destruction: self.0.native_lifetime().map(|lifetime| {
                Arc::new(Destruction(lifetime)) as Arc<dyn ObjectGroupDestructionProof>
            }),
        }
    }
    fn main_route(&self) -> MainWorldRoute {
        MainWorldRoute {
            client: self.0.client(),
            execution_context_id: self.0.main_execution_context_id(),
            unique_context_id: self.0.main_unique_id().into(),
        }
    }
    fn is_current(&self) -> Result<bool, ResolutionWaitError> {
        self.0.is_current().map_err(route_error)
    }
}
pub(super) struct Worlds {
    pub access: ChromiumFrameWorldAccess,
    pub frame: ObjectId,
}
impl PairedWorldSource for Worlds {
    type Pair = Pair;
    type Current<'a> =
        Pin<Box<dyn Future<Output = Result<Arc<Pair>, ResolutionWaitError>> + Send + 'a>>;
    fn current(&self) -> Self::Current<'_> {
        let snapshot = self.access.snapshot(self.frame);
        Box::pin(async move {
            snapshot
                .await
                .map(|pair| Arc::new(Pair(pair)))
                .map_err(route_error)
        })
    }
}

#[cfg(feature = "file-input")]
pub(super) struct UploadRoutes {
    source: Worlds,
    retained: std::sync::Mutex<Option<Arc<Pair>>>,
}
#[cfg(feature = "file-input")]
impl UploadRoutes {
    pub(super) fn new(source: Worlds) -> Self {
        Self {
            source,
            retained: std::sync::Mutex::new(None),
        }
    }
    fn pair(&self, route: &ResolutionRoute) -> Result<Arc<Pair>, ResolutionWaitError> {
        use pwright_composition_chromium_locator_resolution::ResolutionError;
        let guard = self
            .retained
            .lock()
            .map_err(|_| ResolutionError::Setup("upload route lock poisoned"))?;
        let pair = guard
            .as_ref()
            .ok_or(ResolutionError::Setup("upload route missing"))?;
        if !Arc::ptr_eq(&route.client, &pair.0.client())
            || route.utility_world.as_ref() != pair.0.utility_unique_id()
        {
            return Err(ResolutionError::Setup("upload route identity changed").into());
        }
        Ok(pair.clone())
    }
}
#[cfg(feature = "file-input")]
impl pwright_composition_chromium_locator_resolution::ResolutionRouteSource for UploadRoutes {
    type Current<'a> =
        Pin<Box<dyn Future<Output = Result<ResolutionRoute, ResolutionWaitError>> + Send + 'a>>;
    fn current(&self) -> Self::Current<'_> {
        use pwright_composition_chromium_locator_resolution::ResolutionError;
        let future = async {
            self.retained
                .lock()
                .map_err(|_| ResolutionError::Setup("upload route lock poisoned"))?
                .take();
            let pair = self.source.current().await?;
            let route = pair.utility_route();
            *self
                .retained
                .lock()
                .map_err(|_| ResolutionError::Setup("upload route lock poisoned"))? = Some(pair);
            Ok(route)
        };
        if std::mem::size_of_val(&future) > 4096 {
            return Box::pin(std::future::ready(Err(ResolutionError::Setup(
                "upload route future exceeds admitted storage",
            )
            .into())));
        }
        Box::pin(future)
    }
    fn is_current(&self, route: &ResolutionRoute) -> Result<bool, ResolutionWaitError> {
        self.pair(route)?.is_current()
    }
    fn retain_route(
        &self,
        route: &ResolutionRoute,
    ) -> Result<
        Option<pwright_composition_chromium_locator_resolution::ResolutionRouteOwner>,
        ResolutionWaitError,
    > {
        Ok(Some(self.pair(route)?))
    }
}
