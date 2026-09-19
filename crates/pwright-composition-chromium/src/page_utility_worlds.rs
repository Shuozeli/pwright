//! Immediate, bounded selection of existing utility worlds for a Page or Frame.
//! Shares the authoritative route lock and snapshot admission with paired access.
use crate::{
    CdpFrameContextEntry, CdpPageRouteError, CdpPageServiceEntry, CdpServiceRoutes,
    ChromiumFrameWorldAccess, ChromiumNativeWorldLifetime, FrameWorldAccessError,
    frame_world_access::{NATIVE_ID_BYTES, Permit, SNAPSHOT_BYTES},
    native_world_retirement::Identity,
};
use pwright_backend_cdp::ConnectionEpoch;
use pwright_cdp::CdpClient;
use pwright_core::ObjectId;
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    task::{Context, Poll},
};

type Result<T> = std::result::Result<T, FrameWorldAccessError>;

/// Immutable batch: callers cannot drain its owners while keeping an uncharged
/// vector allocation. Empty batches allocate nothing. Worlds are in Frame-ID
/// order, not DOM order; all selection happened under one route publication lock.
pub struct ChromiumPageUtilityWorldSnapshot {
    worlds: Vec<ChromiumUtilityWorldSnapshot>,
}
impl ChromiumPageUtilityWorldSnapshot {
    pub fn worlds(&self) -> &[ChromiumUtilityWorldSnapshot] {
        &self.worlds
    }
}

/// Destruction proof for one exact utility world, not a main/utility pair.
/// Every clone retains its original snapshot admission. Native session detach
/// retires wrappers but never asserts that a borrowed browser world was destroyed.
#[derive(Clone)]
pub struct ChromiumNativeUtilityWorldLifetime(ChromiumNativeWorldLifetime);
impl ChromiumNativeUtilityWorldLifetime {
    pub fn world_destroyed(&self) -> bool {
        self.0.both_worlds_destroyed()
    }
    pub fn native_objects_retired(&self) -> bool {
        self.0.native_objects_retired()
    }
    pub fn poll_session_retired(&mut self, cx: &mut Context<'_>) -> Poll<()> {
        self.0.poll_session_retired(cx)
    }
    pub fn poll_session_release_ready(&self, cx: &mut Context<'_>) -> Poll<()> {
        self.0.poll_session_release_ready(cx)
    }
}

/// Retained existing world; no browser operation, subscription or readiness wait.
/// Keep this owner while using borrowed/cloned native routing values. `is_current`
/// only checks observed routing and is never evidence of native destruction.
pub struct ChromiumUtilityWorldSnapshot {
    epoch: ConnectionEpoch,
    page: ObjectId,
    frame: ObjectId,
    client: Arc<dyn CdpClient>,
    unique: Arc<str>,
    lifetime: Option<ChromiumNativeUtilityWorldLifetime>,
    permit: Arc<Permit>,
    retired: AtomicBool,
}
// A retained native client can keep target/session IDs after route removal;
// the utility receipt adds its owning Page target/session identity. Shared
// unique-ID ownership and fixed Arc/permit control fit the existing allowance.
// Receipt-registry backing is independently included in accessor state bytes.
const _: () = assert!(
    std::mem::size_of::<ChromiumUtilityWorldSnapshot>()
        + std::mem::size_of::<ChromiumPageUtilityWorldSnapshot>()
        + 5 * NATIVE_ID_BYTES
        + 1024
        < SNAPSHOT_BYTES as usize
);
impl ChromiumUtilityWorldSnapshot {
    pub const fn page(&self) -> ObjectId {
        self.page
    }
    pub const fn frame(&self) -> ObjectId {
        self.frame
    }
    pub fn client(&self) -> Arc<dyn CdpClient> {
        self.client.clone()
    }
    pub fn utility_unique_id(&self) -> &str {
        &self.unique
    }
    pub fn native_lifetime(&self) -> Option<ChromiumNativeUtilityWorldLifetime> {
        self.lifetime.clone()
    }
    pub fn is_current(&self) -> Result<bool> {
        let routes = self
            .permit
            .state
            .services
            .routes
            .lock()
            .map_err(|_| CdpPageRouteError::StatePoisoned)?;
        let Some(page) = routes.pages.get(&self.page) else {
            self.retired.store(true, Ordering::Release);
            return Err(CdpPageRouteError::WorldTargetClosed {
                object_id: self.page,
            }
            .into());
        };
        if page.crashed {
            self.retired.store(true, Ordering::Release);
            return Err(CdpPageRouteError::PageCrashed {
                object_id: self.page,
            }
            .into());
        }
        let belongs = routes
            .readiness_frames
            .binary_search_by_key(&self.frame, |(frame, _)| *frame)
            .ok()
            .is_some_and(|index| routes.readiness_frames[index].1 == self.page);
        let current = belongs
            && routes.epoch == Some(self.epoch)
            && routes.frame_contexts.get(&self.frame).is_some_and(|world| {
                !world.page_crashed
                    && world.unique_context_id == self.unique
                    && Arc::ptr_eq(&world.client, &self.client)
            });
        if !current {
            self.retired.store(true, Ordering::Release);
        }
        Ok(current && !self.retired.load(Ordering::Acquire))
    }
}

impl ChromiumFrameWorldAccess {
    /// Select the existing utility world of this exact registered Frame. A
    /// live Frame with no observed utility world returns None immediately;
    /// missing Frame/Page registration or a crashed Page is an error. A Page
    /// ID is never substituted with its main Frame. No main world is required.
    ///
    /// Selection, ID validation and the independent retirement receipt share
    /// the authoritative route lock. One existing 8192-byte snapshot permit is
    /// reserved before cloning native metadata. No browser I/O or readiness wait
    /// is started, and another snapshot of the same world owns a separate
    /// single-waiter receipt rather than cloning this one's waiter authority.
    pub fn existing_frame_utility_world(
        &self,
        frame: ObjectId,
    ) -> Result<Option<ChromiumUtilityWorldSnapshot>> {
        let routes = self
            .0
            .services
            .routes
            .lock()
            .map_err(|_| CdpPageRouteError::StatePoisoned)?;
        let index = routes
            .readiness_frames
            .binary_search_by_key(&frame, |(frame, _)| *frame)
            .map_err(|_| CdpPageRouteError::WorldTargetClosed { object_id: frame })?;
        let page_id = routes.readiness_frames[index].1;
        let page = routes
            .pages
            .get(&page_id)
            .ok_or(CdpPageRouteError::WorldTargetClosed { object_id: frame })?;
        self.validate_utility_page(page, frame)?;
        let Some(world) = routes.frame_contexts.get(&frame) else {
            return Ok(None);
        };
        validate_utility_world(world)?;
        let permit = self.admit()?;
        self.retain_utility_world(&routes, page, page_id, frame, world, permit)
            .map(Some)
    }

    /// Atomically selects existing utility routes of this exact Page (including
    /// local children and OOPIFs). Missing worlds are omitted immediately, even
    /// when their main world is missing too. Never creates worlds or waits for
    /// readiness. `maximum` is 1..=32; overflow fails, never truncates the Page.
    ///
    /// Reserves one shared 8192-byte snapshot slot per selected world BEFORE
    /// allocating the result vector or cloning native identities. This includes
    /// vector element payloads; the batch never exposes mutable vector access.
    /// Each returned owner and lifetime clone retains its slot independently.
    pub fn existing_page_utility_worlds(
        &self,
        page_id: ObjectId,
        maximum: usize,
    ) -> Result<ChromiumPageUtilityWorldSnapshot> {
        if !(1..=32).contains(&maximum) {
            return Err(FrameWorldAccessError::Setup(
                "utility snapshot maximum must be 1..=32",
            ));
        }
        let routes = self
            .0
            .services
            .routes
            .lock()
            .map_err(|_| CdpPageRouteError::StatePoisoned)?;
        let page = routes
            .pages
            .get(&page_id)
            .ok_or(CdpPageRouteError::WorldTargetClosed { object_id: page_id })?;
        self.validate_utility_page(page, page_id)?;
        let selected = || {
            routes
                .readiness_frames
                .iter()
                .filter(|(_, owner)| *owner == page_id)
                .filter_map(|(frame, _)| {
                    routes
                        .frame_contexts
                        .get(frame)
                        .map(|world| (*frame, world))
                })
        };
        let count = selected().count();
        if count > maximum {
            return Err(FrameWorldAccessError::Capacity { limit: maximum });
        }
        // Validate before admitting/cloning. The accessor's construction proves
        // these are utility routes; unlike paired adoption, main is irrelevant.
        for (_, world) in selected() {
            validate_utility_world(world)?;
        }
        let mut permits: [Option<Permit>; 32] = std::array::from_fn(|_| None);
        for permit in &mut permits[..count] {
            *permit = Some(self.admit()?);
        }
        let mut worlds = Vec::with_capacity(count);
        for ((frame, world), permit) in selected().zip(&mut permits[..count]) {
            worlds.push(self.retain_utility_world(
                &routes,
                page,
                page_id,
                frame,
                world,
                permit.take().expect("pre-admitted utility world"),
            )?);
        }
        Ok(ChromiumPageUtilityWorldSnapshot { worlds })
    }

    fn validate_utility_page(&self, page: &CdpPageServiceEntry, target: ObjectId) -> Result<()> {
        if page.crashed {
            return Err(CdpPageRouteError::PageCrashed { object_id: target }.into());
        }
        if self.0.profile.native_retirement {
            crate::validate_route_id(
                "CDP owning Page target ID",
                &page.target_id,
                NATIVE_ID_BYTES,
            )?;
        }
        Ok(())
    }

    fn retain_utility_world(
        &self,
        routes: &CdpServiceRoutes,
        page: &CdpPageServiceEntry,
        page_id: ObjectId,
        frame: ObjectId,
        world: &CdpFrameContextEntry,
        permit: Permit,
    ) -> Result<ChromiumUtilityWorldSnapshot> {
        let permit = Arc::new(permit);
        let registry = self.0.profile.native_retirement.then(|| {
            routes
                .native_world_retirement
                .as_ref()
                .expect("installed native retirement")
        });
        let lifetime = registry
            .map(|registry| {
                registry
                    .register(
                        Identity {
                            epoch: routes.epoch.expect("published topology epoch"),
                            context: page.browser_context_object_id,
                            page_target: page.target_id.as_str().into(),
                            session: world.session_id.as_str().into(),
                            main: None,
                            utility: world.unique_context_id.clone(),
                        },
                        permit.clone(),
                    )
                    .map(ChromiumNativeUtilityWorldLifetime)
            })
            .transpose()?;
        Ok(ChromiumUtilityWorldSnapshot {
            epoch: routes.epoch.expect("published topology epoch"),
            page: page_id,
            frame,
            client: world.client.clone(),
            unique: world.unique_context_id.clone(),
            lifetime,
            permit,
            retired: AtomicBool::new(false),
        })
    }
}

fn validate_utility_world(world: &CdpFrameContextEntry) -> Result<()> {
    for (field, value) in [
        ("CDP target ID", world.target_id.as_str()),
        ("CDP session ID", world.session_id.as_str()),
        (
            "CDP utility world unique ID",
            world.unique_context_id.as_ref(),
        ),
    ] {
        crate::validate_route_id(field, value, NATIVE_ID_BYTES)?;
    }
    Ok(())
}
