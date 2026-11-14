//! Optional Page media emulation through the exact published Chromium route.
//!
//! The leaf owns one fixed, resource-accounted Page override table. It starts
//! no task, browser process, or event queue. One admitted host operation issues
//! one bounded CDP command on the Page's current session; language bindings
//! only adapt Playwright-shaped enum strings.

use std::{error::Error, sync::Mutex};

use pwright_backend_cdp_topology::CdpContextOwnership;
use pwright_cdp::CdpSession;
use pwright_core::{
    ObjectId, ProductionResourceLedger, ResourceClass, ResourceLease, ResourceRequirements,
    ResourceScope,
};
use pwright_plugin_chromium_session_initialization::{
    ChromiumSessionInitializationFuture, ChromiumSessionInitializationTarget,
    ChromiumSessionInitializer,
};
use serde::Serialize;

use crate::{HostError, HostSession};

const MAX_PAGES: usize = 64;
#[cfg(feature = "context-forced-colors")]
const MAX_CONTEXTS: usize = 16;
const STORAGE_BYTES: u64 = 16 * 1024;
const SCOPE: ResourceScope = ResourceScope::extension(132, 1);

pub(crate) fn requirements() -> ResourceRequirements {
    ResourceRequirements::none()
        .with_class(
            ResourceClass::BackendStateBytes,
            STORAGE_BYTES,
            STORAGE_BYTES,
        )
        .with_metadata(1, 1)
}

pub(crate) struct Installation {
    gate: tokio::sync::Mutex<()>,
    pages: Mutex<[Option<PageState>; MAX_PAGES]>,
    #[cfg(feature = "context-forced-colors")]
    contexts: Mutex<[Option<ContextState>; MAX_CONTEXTS]>,
    default_options: Option<PersistentContextMediaOptions>,
    _storage: ResourceLease,
}

impl Installation {
    pub(crate) fn new_with_default(
        resources: &ProductionResourceLedger,
        default_options: Option<PersistentContextMediaOptions>,
    ) -> Result<Self, HostError> {
        let ledger = resources
            .claim(requirements())
            .map_err(HostError::backend)?;
        let storage = ledger
            .reserve(SCOPE, ResourceClass::BackendStateBytes, STORAGE_BYTES)
            .map_err(HostError::backend)?;
        Ok(Self {
            gate: tokio::sync::Mutex::new(()),
            pages: Mutex::new([None; MAX_PAGES]),
            #[cfg(feature = "context-forced-colors")]
            contexts: Mutex::new([None; MAX_CONTEXTS]),
            default_options,
            _storage: storage,
        })
    }

    fn initial_state(
        &self,
        page: ObjectId,
        context: ObjectId,
        ownership: CdpContextOwnership,
    ) -> Result<Option<PageState>, HostError> {
        if let Some(state) = self
            .pages
            .lock()
            .map_err(HostError::backend)?
            .iter()
            .flatten()
            .find(|state| state.page == page)
            .copied()
        {
            return Ok(Some(state));
        }
        #[cfg(feature = "context-forced-colors")]
        if ownership == CdpContextOwnership::Owned {
            let contexts = self.contexts.lock().map_err(HostError::backend)?;
            return Ok(contexts
                .iter()
                .flatten()
                .find(|state| state.context == context)
                .map(|state| PageState::from_context(page, *state)));
        }
        #[cfg(not(feature = "context-forced-colors"))]
        let _ = context;
        Ok(match ownership {
            CdpContextOwnership::Default => self
                .default_options
                .map(|options| PageState::from_persistent(page, options)),
            CdpContextOwnership::Owned | CdpContextOwnership::External => None,
        })
    }

    #[cfg(feature = "context-forced-colors")]
    pub(crate) fn install_context(
        &self,
        context: ObjectId,
        forced_colors: ContextForcedColors,
    ) -> Result<(), HostError> {
        let mut contexts = self.contexts.lock().map_err(HostError::backend)?;
        if contexts
            .iter()
            .flatten()
            .any(|state| state.context == context)
        {
            return Err(HostError::InvalidArgument(
                "forced colors policy is already installed for this Context".into(),
            ));
        }
        let slot = contexts
            .iter_mut()
            .find(|slot| slot.is_none())
            .ok_or_else(|| {
                HostError::ResourceExhausted(format!(
                    "forced colors policy exceeds {MAX_CONTEXTS} live Contexts"
                ))
            })?;
        *slot = Some(ContextState {
            context,
            forced_colors: forced_colors.0,
        });
        Ok(())
    }

    #[cfg(feature = "context-forced-colors")]
    pub(crate) fn reconcile_contexts(
        &self,
        mut is_live: impl FnMut(ObjectId) -> bool,
    ) -> Result<(), HostError> {
        let mut contexts = self.contexts.lock().map_err(HostError::backend)?;
        for slot in contexts.iter_mut() {
            if slot.is_some_and(|state| !is_live(state.context)) {
                *slot = None;
            }
        }
        drop(contexts);
        let mut pages = self.pages.lock().map_err(HostError::backend)?;
        for slot in pages.iter_mut() {
            if slot.is_some_and(|state| state.context.is_some_and(|context| !is_live(context))) {
                *slot = None;
            }
        }
        Ok(())
    }

    #[cfg(feature = "context-forced-colors")]
    pub(crate) fn remove_context(&self, context: ObjectId) -> Result<(), HostError> {
        let mut contexts = self.contexts.lock().map_err(HostError::backend)?;
        for slot in contexts.iter_mut() {
            if slot.is_some_and(|state| state.context == context) {
                *slot = None;
            }
        }
        let mut pages = self.pages.lock().map_err(HostError::backend)?;
        for slot in pages.iter_mut() {
            if slot.is_some_and(|state| state.context == Some(context)) {
                *slot = None;
            }
        }
        Ok(())
    }

    #[cfg(feature = "context-forced-colors")]
    pub(crate) fn remove_page(&self, page: ObjectId) -> Result<(), HostError> {
        let mut pages = self.pages.lock().map_err(HostError::backend)?;
        for slot in pages.iter_mut() {
            if slot.is_some_and(|state| state.page == page) {
                *slot = None;
            }
        }
        Ok(())
    }

    fn prepare(
        &self,
        page: ObjectId,
        context: Option<ObjectId>,
        active: &[Option<ObjectId>; MAX_PAGES],
        options: MediaOptions<'_>,
    ) -> Result<PageState, HostError> {
        #[cfg(feature = "context-forced-colors")]
        let context_default = context
            .map(|context| {
                self.contexts
                    .lock()
                    .map_err(HostError::backend)
                    .map(|contexts| {
                        contexts
                            .iter()
                            .flatten()
                            .find(|state| state.context == context)
                            .map(|state| PageState::from_context(page, *state))
                    })
            })
            .transpose()?
            .flatten();
        #[cfg(not(feature = "context-forced-colors"))]
        let _ = context;
        let mut pages = self.pages.lock().map_err(HostError::backend)?;
        for entry in pages.iter_mut() {
            if entry.is_some_and(|entry| !active.contains(&Some(entry.page))) {
                *entry = None;
            }
        }
        let previous = match pages
            .iter()
            .find(|entry| entry.is_some_and(|entry| entry.page == page))
        {
            Some(entry) => *entry,
            None if pages.iter().any(Option::is_none) => None,
            None => {
                return Err(HostError::ResourceExhausted(
                    "media override Page capacity exceeds 64".into(),
                ));
            }
        };
        #[cfg(feature = "context-forced-colors")]
        let mut state = previous.or(context_default).unwrap_or(PageState::new(page));
        #[cfg(not(feature = "context-forced-colors"))]
        let mut state = previous.unwrap_or(PageState::new(page));
        state.apply(options)?;
        Ok(state)
    }

    fn commit(&self, state: PageState) -> Result<(), HostError> {
        let mut pages = self.pages.lock().map_err(HostError::backend)?;
        let slot = match pages
            .iter_mut()
            .find(|entry| entry.is_some_and(|entry| entry.page == state.page))
        {
            Some(entry) => entry,
            None => pages
                .iter_mut()
                .find(|entry| entry.is_none())
                .ok_or_else(|| {
                    HostError::ResourceExhausted("media override Page capacity exceeds 64".into())
                })?,
        };
        *slot = Some(state);
        Ok(())
    }
}

const _: () = assert!(
    std::mem::size_of::<Installation>()
        + std::mem::size_of::<[Option<PageState>; MAX_PAGES]>()
        + {
            #[cfg(feature = "context-forced-colors")]
            {
                std::mem::size_of::<[Option<ContextState>; MAX_CONTEXTS]>()
            }
            #[cfg(not(feature = "context-forced-colors"))]
            {
                0
            }
        }
        + 2 * std::mem::size_of::<usize>()
        + std::mem::size_of::<pwright_core::ResourceRequest>()
        < STORAGE_BYTES as usize
);

#[derive(Clone, Copy)]
enum MediaType {
    Screen,
    Print,
}

#[derive(Clone, Copy)]
enum ColorScheme {
    Light,
    Dark,
    NoPreference,
}

#[derive(Clone, Copy)]
enum ReducedMotion {
    Reduce,
    NoPreference,
}

#[derive(Clone, Copy)]
enum ForcedColors {
    Active,
    None,
}

#[cfg(feature = "context-forced-colors")]
#[derive(Clone, Copy)]
pub struct ContextForcedColors(ForcedColors);

#[cfg(feature = "context-forced-colors")]
impl ContextForcedColors {
    pub fn new(value: &str) -> Result<Self, HostError> {
        match value {
            "active" => Ok(Self(ForcedColors::Active)),
            "none" => Ok(Self(ForcedColors::None)),
            _ => Err(HostError::InvalidArgument(format!(
                "Browser.new_context forced_colors value {value:?} is not supported"
            ))),
        }
    }
}

#[cfg(feature = "context-forced-colors")]
#[derive(Clone, Copy)]
struct ContextState {
    context: ObjectId,
    forced_colors: ForcedColors,
}

#[derive(Clone, Copy)]
enum Contrast {
    More,
    NoPreference,
}

/// Immutable media defaults admitted before an owned persistent Chromium
/// Context publishes its first Page. Values are parsed once in Rust and retain
/// no strings, protocol handles or language-runtime objects.
#[derive(Clone, Copy)]
pub struct PersistentContextMediaOptions {
    color_scheme: Option<ColorScheme>,
    reduced_motion: Option<ReducedMotion>,
    contrast: Option<Contrast>,
}

impl PersistentContextMediaOptions {
    pub fn new(
        color_scheme: Option<&str>,
        reduced_motion: Option<&str>,
        contrast: Option<&str>,
    ) -> Result<Self, HostError> {
        let mut state = PageState::new(ObjectId::new(1));
        state.apply(MediaOptions {
            media: None,
            color_scheme,
            reduced_motion,
            forced_colors: None,
            contrast,
        })?;
        Ok(Self {
            color_scheme: state.color_scheme,
            reduced_motion: state.reduced_motion,
            contrast: state.contrast,
        })
    }
}

#[derive(Clone, Copy)]
struct PageState {
    page: ObjectId,
    #[cfg(feature = "context-forced-colors")]
    context: Option<ObjectId>,
    media: Option<MediaType>,
    color_scheme: Option<ColorScheme>,
    reduced_motion: Option<ReducedMotion>,
    forced_colors: Option<ForcedColors>,
    contrast: Option<Contrast>,
}

impl PageState {
    const fn new(page: ObjectId) -> Self {
        Self {
            page,
            #[cfg(feature = "context-forced-colors")]
            context: None,
            media: None,
            color_scheme: None,
            reduced_motion: None,
            forced_colors: None,
            contrast: None,
        }
    }

    const fn from_persistent(page: ObjectId, options: PersistentContextMediaOptions) -> Self {
        Self {
            page,
            #[cfg(feature = "context-forced-colors")]
            context: None,
            media: None,
            color_scheme: options.color_scheme,
            reduced_motion: options.reduced_motion,
            forced_colors: None,
            contrast: options.contrast,
        }
    }

    #[cfg(feature = "context-forced-colors")]
    const fn from_context(page: ObjectId, options: ContextState) -> Self {
        Self {
            page,
            context: Some(options.context),
            media: None,
            color_scheme: None,
            reduced_motion: None,
            forced_colors: Some(options.forced_colors),
            contrast: None,
        }
    }

    fn apply(&mut self, options: MediaOptions<'_>) -> Result<(), HostError> {
        if let Some(value) = options.media {
            self.media = nullable("media", value, |value| match value {
                "screen" => Some(MediaType::Screen),
                "print" => Some(MediaType::Print),
                _ => None,
            })?;
        }
        if let Some(value) = options.color_scheme {
            self.color_scheme = nullable("color_scheme", value, |value| match value {
                "light" => Some(ColorScheme::Light),
                "dark" => Some(ColorScheme::Dark),
                "no-preference" => Some(ColorScheme::NoPreference),
                _ => None,
            })?;
        }
        if let Some(value) = options.reduced_motion {
            self.reduced_motion = nullable("reduced_motion", value, |value| match value {
                "reduce" => Some(ReducedMotion::Reduce),
                "no-preference" => Some(ReducedMotion::NoPreference),
                _ => None,
            })?;
        }
        if let Some(value) = options.forced_colors {
            self.forced_colors = nullable("forced_colors", value, |value| match value {
                "active" => Some(ForcedColors::Active),
                "none" => Some(ForcedColors::None),
                _ => None,
            })?;
        }
        if let Some(value) = options.contrast {
            self.contrast = nullable("contrast", value, |value| match value {
                "more" => Some(Contrast::More),
                "no-preference" => Some(Contrast::NoPreference),
                _ => None,
            })?;
        }
        Ok(())
    }

    fn params(self) -> MediaParams {
        MediaParams {
            media: match self.media {
                Some(MediaType::Screen) => "screen",
                Some(MediaType::Print) => "print",
                None => "",
            },
            features: [
                MediaFeature {
                    name: "prefers-color-scheme",
                    value: match self.color_scheme {
                        Some(ColorScheme::Light) | None => "light",
                        Some(ColorScheme::Dark) => "dark",
                        Some(ColorScheme::NoPreference) => "no-preference",
                    },
                },
                MediaFeature {
                    name: "prefers-reduced-motion",
                    value: match self.reduced_motion {
                        Some(ReducedMotion::Reduce) => "reduce",
                        Some(ReducedMotion::NoPreference) | None => "no-preference",
                    },
                },
                MediaFeature {
                    name: "forced-colors",
                    value: match self.forced_colors {
                        Some(ForcedColors::Active) => "active",
                        Some(ForcedColors::None) | None => "none",
                    },
                },
                MediaFeature {
                    name: "prefers-contrast",
                    value: match self.contrast {
                        Some(Contrast::More) => "more",
                        Some(Contrast::NoPreference) | None => "no-preference",
                    },
                },
            ],
        }
    }
}

pub(crate) struct Initializer<Downstream> {
    downstream: Downstream,
    installation: std::sync::Arc<Installation>,
    connection: std::sync::Arc<pwright_cdp::CdpConnection>,
}

impl<Downstream> Initializer<Downstream> {
    pub(crate) fn new(
        downstream: Downstream,
        installation: std::sync::Arc<Installation>,
        connection: std::sync::Arc<pwright_cdp::CdpConnection>,
    ) -> Self {
        Self {
            downstream,
            installation,
            connection,
        }
    }
}

impl<Downstream> ChromiumSessionInitializer for Initializer<Downstream>
where
    Downstream: ChromiumSessionInitializer,
{
    fn initialize<'a>(
        &'a self,
        session_id: &'a str,
        target_id: &'a str,
        ownership: CdpContextOwnership,
        is_page: bool,
    ) -> ChromiumSessionInitializationFuture<'a> {
        self.downstream
            .initialize(session_id, target_id, ownership, is_page)
    }

    fn initialize_scoped<'a>(
        &'a self,
        target: ChromiumSessionInitializationTarget<'a>,
    ) -> ChromiumSessionInitializationFuture<'a> {
        Box::pin(async move {
            self.downstream.initialize_scoped(target).await?;
            let _gate = self.installation.gate.lock().await;
            let Some(state) =
                self.installation
                    .initial_state(target.page, target.context, target.ownership)?
            else {
                return Ok(());
            };
            CdpSession::new(
                std::sync::Arc::clone(&self.connection),
                target.session_id.to_owned(),
                target.target_id.to_owned(),
            )
            .send(
                "Emulation.setEmulatedMedia",
                serde_json::to_value(state.params())?,
            )
            .await?;
            #[cfg(feature = "context-forced-colors")]
            if state.context.is_none() {
                self.installation.commit(state)?;
            }
            #[cfg(not(feature = "context-forced-colors"))]
            self.installation.commit(state)?;
            Ok::<(), Box<dyn Error + Send + Sync>>(())
        })
    }
}

fn nullable<T>(
    field: &'static str,
    value: &str,
    parse: impl FnOnce(&str) -> Option<T>,
) -> Result<Option<T>, HostError> {
    if value == "null" {
        return Ok(None);
    }
    parse(value).map(Some).ok_or_else(|| {
        HostError::InvalidArgument(format!(
            "Page.emulate_media {field} value {value:?} is not supported"
        ))
    })
}

#[derive(Serialize)]
struct MediaParams {
    media: &'static str,
    features: [MediaFeature; 4],
}

#[derive(Serialize)]
struct MediaFeature {
    name: &'static str,
    value: &'static str,
}

#[derive(Clone, Copy)]
struct MediaOptions<'a> {
    media: Option<&'a str>,
    color_scheme: Option<&'a str>,
    reduced_motion: Option<&'a str>,
    forced_colors: Option<&'a str>,
    contrast: Option<&'a str>,
}

impl HostSession {
    pub async fn page_emulate_media(
        &self,
        page_id: &str,
        media: Option<&str>,
        color_scheme: Option<&str>,
        reduced_motion: Option<&str>,
        forced_colors: Option<&str>,
        contrast: Option<&str>,
    ) -> Result<(), HostError> {
        let (page, _call, _operation) = self.admit_operation(page_id)?;
        let installation = {
            let state = self.state.lock().map_err(HostError::backend)?;
            let live = state.as_ref().ok_or(HostError::TargetClosed)?;
            live.emulate_media.clone()
        };
        let _gate = tokio::select! {
            biased;
            () = self.stop.cancelled() => return Err(HostError::TargetClosed),
            gate = installation.gate.lock() => gate,
        };
        let (connection, session_id, target_id, context, active) = {
            let state = self.state.lock().map_err(HostError::backend)?;
            let live = state.as_ref().ok_or(HostError::TargetClosed)?;
            let topology = live.composition.topology().map_err(HostError::backend)?;
            let topology = topology.topology();
            let route = topology
                .page_session_routes()
                .find(|route| route.object_id() == page)
                .ok_or(HostError::TargetClosed)?;
            let session_id = route.session_id().to_owned();
            let target_id = route.target_id().to_owned();
            #[cfg(feature = "context-forced-colors")]
            let context = Some(route.browser_context_object_id());
            #[cfg(not(feature = "context-forced-colors"))]
            let context = None;
            let mut active = [None; MAX_PAGES];
            for (slot, route) in active.iter_mut().zip(topology.page_session_routes()) {
                *slot = Some(route.object_id());
            }
            (
                live.composition.connection().clone(),
                session_id,
                target_id,
                context,
                active,
            )
        };
        let next = installation.prepare(
            page,
            context,
            &active,
            MediaOptions {
                media,
                color_scheme,
                reduced_motion,
                forced_colors,
                contrast,
            },
        )?;
        let params = serde_json::to_value(next.params()).map_err(HostError::backend)?;
        let session = CdpSession::new(connection, session_id, target_id);
        session
            .send("Emulation.setEmulatedMedia", params)
            .await
            .map_err(HostError::backend)?;
        installation.commit(next)
    }

    #[cfg(feature = "context-forced-colors")]
    pub(crate) fn retire_forced_colors_page(&self, page_id: &str) -> Result<(), HostError> {
        let page = page_id
            .parse::<u64>()
            .ok()
            .filter(|id| *id != 0)
            .map(ObjectId::new)
            .ok_or_else(|| HostError::InvalidArgument("invalid portable Page ID".into()))?;
        let installation = {
            let state = self.state.lock().map_err(HostError::backend)?;
            state
                .as_ref()
                .ok_or(HostError::TargetClosed)?
                .emulate_media
                .clone()
        };
        installation.remove_page(page)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_resources() -> ProductionResourceLedger {
        let required = requirements();
        let mut limits = pwright_core::ResourceLimits::deny_all(required.scopes, required.leases);
        for class in ResourceClass::ALL {
            let limit = required.class(class);
            limits = limits.with_class(class, limit.global, limit.per_scope);
        }
        ProductionResourceLedger::new(limits, required).unwrap()
    }

    #[test]
    fn later_calls_preserve_unspecified_overrides_and_null_resets_one_field() {
        // Arrange
        let page = ObjectId::new(7);
        let mut state = PageState::new(page);
        state
            .apply(MediaOptions {
                media: Some("print"),
                color_scheme: Some("dark"),
                reduced_motion: None,
                forced_colors: None,
                contrast: Some("more"),
            })
            .unwrap();

        // Act
        state
            .apply(MediaOptions {
                media: None,
                color_scheme: None,
                reduced_motion: Some("reduce"),
                forced_colors: None,
                contrast: Some("null"),
            })
            .unwrap();
        let value = serde_json::to_value(state.params()).unwrap();

        // Assert
        assert_eq!(
            value,
            serde_json::json!({
                "media": "print",
                "features": [
                    { "name": "prefers-color-scheme", "value": "dark" },
                    { "name": "prefers-reduced-motion", "value": "reduce" },
                    { "name": "forced-colors", "value": "none" },
                    { "name": "prefers-contrast", "value": "no-preference" },
                ]
            })
        );
    }

    #[test]
    fn invalid_enum_is_rejected_before_browser_work() {
        // Arrange
        let mut state = PageState::new(ObjectId::new(7));
        let options = MediaOptions {
            media: None,
            color_scheme: None,
            reduced_motion: Some("sometimes"),
            forced_colors: None,
            contrast: None,
        };

        // Act
        let result = state.apply(options);

        // Assert
        assert!(matches!(result, Err(HostError::InvalidArgument(_))));
    }

    #[test]
    fn persistent_defaults_apply_only_to_the_default_context() {
        // Arrange
        let resources = test_resources();
        let options =
            PersistentContextMediaOptions::new(Some("dark"), Some("reduce"), Some("more")).unwrap();
        let installation = Installation::new_with_default(&resources, Some(options)).unwrap();

        // Act
        let default = installation
            .initial_state(
                ObjectId::new(7),
                ObjectId::new(1),
                CdpContextOwnership::Default,
            )
            .unwrap();
        let owned = installation
            .initial_state(
                ObjectId::new(8),
                ObjectId::new(2),
                CdpContextOwnership::Owned,
            )
            .unwrap();
        let external = installation
            .initial_state(
                ObjectId::new(9),
                ObjectId::new(3),
                CdpContextOwnership::External,
            )
            .unwrap();

        // Assert
        assert_eq!(
            serde_json::to_value(default.unwrap().params()).unwrap(),
            serde_json::json!({
                "media": "",
                "features": [
                    { "name": "prefers-color-scheme", "value": "dark" },
                    { "name": "prefers-reduced-motion", "value": "reduce" },
                    { "name": "forced-colors", "value": "none" },
                    { "name": "prefers-contrast", "value": "more" },
                ]
            })
        );
        assert!(owned.is_none());
        assert!(external.is_none());
    }

    #[test]
    fn persistent_media_rejects_unknown_values_before_native_work() {
        // Arrange
        let invalid = Some("sepia");

        // Act
        let result = PersistentContextMediaOptions::new(invalid, None, None);

        // Assert
        assert!(matches!(result, Err(HostError::InvalidArgument(_))));
    }

    #[cfg(feature = "context-forced-colors")]
    #[test]
    fn context_policy_applies_to_future_page_and_oopif_but_page_override_wins() {
        // Arrange
        let resources = test_resources();
        let installation = Installation::new_with_default(&resources, None).unwrap();
        let context = ObjectId::new(11);
        let first_page = ObjectId::new(21);
        let second_page = ObjectId::new(22);
        let active = std::array::from_fn(|index| match index {
            0 => Some(first_page),
            1 => Some(second_page),
            _ => None,
        });
        installation
            .install_context(context, ContextForcedColors::new("active").unwrap())
            .unwrap();

        // Act: Context defaults do not occupy Page override slots. Both the
        // Page and its OOPIF independently acquire that same Context default.
        let initial = installation
            .initial_state(first_page, context, CdpContextOwnership::Owned)
            .unwrap()
            .unwrap();
        let oopif = installation
            .initial_state(first_page, context, CdpContextOwnership::Owned)
            .unwrap()
            .unwrap();
        assert!(
            installation
                .pages
                .lock()
                .unwrap()
                .iter()
                .all(Option::is_none)
        );
        let overridden = installation
            .prepare(
                first_page,
                Some(context),
                &active,
                MediaOptions {
                    media: None,
                    color_scheme: None,
                    reduced_motion: None,
                    forced_colors: Some("none"),
                    contrast: None,
                },
            )
            .unwrap();
        installation.commit(overridden).unwrap();
        let later_oopif = installation
            .initial_state(first_page, context, CdpContextOwnership::Owned)
            .unwrap()
            .unwrap();
        let future_page = installation
            .initial_state(second_page, context, CdpContextOwnership::Owned)
            .unwrap()
            .unwrap();

        // Assert
        let forced = |state: PageState| {
            serde_json::to_value(state.params()).unwrap()["features"][2]["value"].clone()
        };
        assert_eq!(forced(initial), "active");
        assert_eq!(forced(oopif), "active");
        assert_eq!(forced(later_oopif), "none");
        assert_eq!(forced(future_page), "active");
        assert!(
            installation
                .initial_state(
                    ObjectId::new(23),
                    ObjectId::new(12),
                    CdpContextOwnership::External
                )
                .unwrap()
                .is_none()
        );
    }

    #[cfg(feature = "context-forced-colors")]
    #[test]
    fn context_policy_is_reclaimed_on_close_and_rejects_invalid_values() {
        // Arrange
        let resources = test_resources();
        let installation = Installation::new_with_default(&resources, None).unwrap();
        let first = ObjectId::new(101);
        let page = ObjectId::new(201);
        installation
            .install_context(first, ContextForcedColors::new("active").unwrap())
            .unwrap();
        let state = installation
            .initial_state(page, first, CdpContextOwnership::Owned)
            .unwrap()
            .unwrap();
        installation.commit(state).unwrap();

        // Act
        let duplicate =
            installation.install_context(first, ContextForcedColors::new("none").unwrap());
        installation.remove_context(first).unwrap();
        let after_close = installation
            .initial_state(page, first, CdpContextOwnership::Owned)
            .unwrap();
        let invalid = ContextForcedColors::new("yellow");

        // Assert
        assert!(matches!(duplicate, Err(HostError::InvalidArgument(_))));
        assert!(after_close.is_none());
        assert!(matches!(invalid, Err(HostError::InvalidArgument(_))));
    }

    #[cfg(feature = "context-forced-colors")]
    #[test]
    fn context_policy_capacity_is_fixed_and_reusable() {
        // Arrange
        let resources = test_resources();
        let installation = Installation::new_with_default(&resources, None).unwrap();
        let active = ContextForcedColors::new("active").unwrap();
        for id in 1..=MAX_CONTEXTS {
            installation
                .install_context(ObjectId::new(id as u64), active)
                .unwrap();
        }

        // Act
        let exhausted = installation.install_context(ObjectId::new(17), active);
        installation.remove_context(ObjectId::new(1)).unwrap();
        let reused = installation.install_context(ObjectId::new(17), active);

        // Assert
        assert!(matches!(exhausted, Err(HostError::ResourceExhausted(_))));
        assert!(reused.is_ok());
    }

    #[cfg(feature = "context-forced-colors")]
    #[test]
    fn native_retirement_reclaims_policy_after_close_waiter_is_canceled() {
        // Arrange: a canceled close waiter did not reach remove_context, but
        // native topology no longer contains Context 1.
        let resources = test_resources();
        let installation = Installation::new_with_default(&resources, None).unwrap();
        let active = ContextForcedColors::new("active").unwrap();
        for id in 1..=MAX_CONTEXTS {
            installation
                .install_context(ObjectId::new(id as u64), active)
                .unwrap();
        }
        let retired = ObjectId::new(1);
        let stale_page = ObjectId::new(201);
        let stale_override = installation
            .initial_state(stale_page, retired, CdpContextOwnership::Owned)
            .unwrap()
            .unwrap();
        installation.commit(stale_override).unwrap();

        // Act: the next policy admission checks authoritative native liveness.
        installation
            .reconcile_contexts(|context| context != retired)
            .unwrap();
        let replacement = ObjectId::new(17);
        let admitted = installation.install_context(replacement, active);

        // Assert: both the dead Context slot and its Page override are gone;
        // live policies stay installed and the fixed capacity can be reused.
        assert!(admitted.is_ok());
        assert!(
            installation
                .initial_state(stale_page, retired, CdpContextOwnership::Owned)
                .unwrap()
                .is_none()
        );
        assert!(
            installation
                .pages
                .lock()
                .unwrap()
                .iter()
                .all(|slot| slot.is_none_or(|state| state.page != stale_page))
        );
        assert!(
            installation
                .initial_state(
                    ObjectId::new(202),
                    ObjectId::new(2),
                    CdpContextOwnership::Owned
                )
                .unwrap()
                .is_some()
        );
        assert!(
            installation
                .initial_state(ObjectId::new(203), replacement, CdpContextOwnership::Owned)
                .unwrap()
                .is_some()
        );
    }

    #[cfg(feature = "context-forced-colors")]
    #[test]
    fn page_override_slot_is_reclaimed_on_page_close() {
        // Arrange
        let resources = test_resources();
        let installation = Installation::new_with_default(&resources, None).unwrap();
        let context = ObjectId::new(11);
        let page = ObjectId::new(21);
        installation
            .install_context(context, ContextForcedColors::new("active").unwrap())
            .unwrap();
        let active = std::array::from_fn(|index| (index == 0).then_some(page));
        let override_state = installation
            .prepare(
                page,
                Some(context),
                &active,
                MediaOptions {
                    media: None,
                    color_scheme: None,
                    reduced_motion: None,
                    forced_colors: Some("none"),
                    contrast: None,
                },
            )
            .unwrap();
        installation.commit(override_state).unwrap();

        // Act
        installation.remove_page(page).unwrap();
        let replacement = installation
            .initial_state(page, context, CdpContextOwnership::Owned)
            .unwrap()
            .unwrap();

        // Assert
        assert_eq!(
            serde_json::to_value(replacement.params()).unwrap()["features"][2]["value"],
            "active"
        );
        assert!(
            installation
                .pages
                .lock()
                .unwrap()
                .iter()
                .all(Option::is_none)
        );
    }
}
