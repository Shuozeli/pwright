use std::num::NonZeroUsize;
use std::time::Duration;

use pwright_backend_cdp_context_creation::CdpContextCreationLimits;
use pwright_backend_cdp_dom_content::CdpDomContentProductionProfile;
use pwright_backend_cdp_evaluation::{CdpEvaluationLimits, CdpEvaluationProductionProfile};
use pwright_backend_cdp_locator::CdpLocatorProductionProfile;
use pwright_backend_cdp_navigation::{CdpNavigationLimits, CdpNavigationProductionProfile};
use pwright_backend_cdp_page_creation::CdpPageCreationLimits;
use pwright_backend_cdp_page_lifecycle::CdpPageCloseLimits;
use pwright_backend_cdp_topology::{CdpTopologyLimits, CdpTopologyProductionProfile};
use pwright_cdp::CdpConnectionProductionProfile;
use pwright_cdp::connection::ConnectionConfig;
use pwright_composition_chromium::{
    CapabilityWireProfile, CdpPageRouteProductionProfile, ChromiumAttachFactory,
    ChromiumAttachProductionProfile, ChromiumExecutionWorldReadinessProfile,
    ChromiumPageDiscoveryProductionProfile, ChromiumTopologyEventDriverProductionProfile,
    ChromiumUtilityWorldProfile, ContentInstallationProfile, ContextCreationInstallationProfile,
    ContextLifecycleInstallationProfile, DomContentInstallationProfile,
    EvaluationInstallationProfile, LocatorInstallationProfile, NavigationInstallationProfile,
    PageCreationInstallationProfile, PageLifecycleInstallationProfile,
};
use pwright_core::{ObjectId, ProtocolVersion, ResourceClass, ResourceLimits, ResourceScope};
use pwright_runtime::RuntimeLimits;
use pwright_server_wire::ServerSupervisorLimits;
use pwright_transport::FrameLimits;

use crate::{HostError, evaluation};

/// Explicit initial compatibility profile; no installer or browser launcher.
pub fn attach_factory(timeout: Duration) -> Result<ChromiumAttachFactory, HostError> {
    attach_factory_with_native_close(timeout, false)
}

pub(crate) fn attach_factory_with_native_close(
    timeout: Duration,
    native_close: bool,
) -> Result<ChromiumAttachFactory, HostError> {
    let connection = CdpConnectionProductionProfile::new(
        ResourceScope::Connection,
        ConnectionConfig {
            timeout,
            write_channel_capacity: 32,
            // Initial Runtime.enable announces existing worlds in a burst
            // while target initialization is still awaiting command replies.
            // This is a finite profile budget, not an unbounded event queue.
            event_channel_capacity: 1024,
            // Preserve no-delay Mouse batch initiation for every admitted native
            // operation, plus the existing non-Mouse allowance. Write queue
            // backpressure is unchanged; no serializing click scheduler is added.
            pending_command_capacity: 32 + crate::mouse::MAX_PENDING_COMMANDS,
            max_command_bytes: 4 * 1024 * 1024,
            max_response_bytes: 4 * 1024 * 1024,
            max_event_bytes: 256 * 1024,
            max_websocket_bytes: 4 * 1024 * 1024,
            max_protocol_error_bytes: 4096,
        },
    )
    .with_pending_session_retirement(256)
    .map_err(HostError::backend)?
    // This host installs only main/Pwright utility-world consumers. Preserve
    // all other event methods and malformed/unknown world announcements.
    .with_event_admission_policy(
        pwright_composition_chromium::chromium_frame_world_event_admission(),
    )
    .map_err(HostError::backend)?;
    let topology = CdpTopologyProductionProfile::new(
        CdpTopologyLimits::new(64, 16, 16, 64, 128, 256),
        ResourceScope::extension(120, 1),
    )
    .map_err(HostError::backend)?;
    #[cfg(feature = "file-input")]
    // Keep ordinary connection limits unchanged when selecting uploads.
    let connection = connection
        .with_large_commands(crate::file_input::profile::large_commands()?)
        .map_err(HostError::backend)?;
    let routes =
        CdpPageRouteProductionProfile::from_topology(ResourceScope::extension(120, 2), topology);
    let events = ChromiumTopologyEventDriverProductionProfile::from_profiles(
        ResourceScope::extension(120, 3),
        &connection,
        topology,
    )
    .and_then(ChromiumTopologyEventDriverProductionProfile::with_parallel_recursive_attachment)
    .map_err(HostError::backend)?;
    let discovery = ChromiumPageDiscoveryProductionProfile::from_profiles(&connection, topology);
    let runtime_limits = RuntimeLimits::new(256, 16).with_active_commands(8);
    #[cfg(feature = "concurrent-close")]
    let runtime_limits = if native_close {
        RuntimeLimits::new(crate::close_jobs::MAX_OBJECTS, crate::close_jobs::MAX_CALLS)
            .with_active_commands(8)
    } else {
        runtime_limits
    };
    let profile = ChromiumAttachProductionProfile::new(
        connection,
        topology,
        routes,
        runtime_limits,
        ServerSupervisorLimits::new(8).map_err(HostError::backend)?,
    )
    .with_controlled_target_concurrency(NonZeroUsize::new(8).expect("fixed positive concurrency"))
    // Current plus retained/replaced Page state; independent of Locator selection.
    .with_page_input(
        pwright_composition_chromium::CdpInputProductionProfile::new(
            ResourceScope::extension(120, 17),
            pwright_composition_chromium::CdpInputPolicy::Interleaved,
            32,
        ),
    )
    .with_utility_world(ChromiumUtilityWorldProfile::playwright_compatible())
    .with_execution_world_readiness(ChromiumExecutionWorldReadinessProfile::from_topology(
        ResourceScope::extension(120, 9),
        topology,
        NonZeroUsize::new(8).expect("fixed positive world-wait capacity"),
    ))
    .with_content(ContentInstallationProfile::new(
        pwright_backend_cdp_content::CdpScreenshotProductionProfile::new(
            crate::screenshot::MAX_BYTES,
            2,
        )
        .with_viewport_clip_intersection(),
        pwright_codec_content::ContentCodecLimits::new(crate::screenshot::MAX_BYTES, 4096),
        CapabilityWireProfile::new(
            ProtocolVersion::new(1, 0),
            FrameLimits::new(
                16 * 1024,
                1,
                crate::screenshot::MAX_BYTES,
                crate::screenshot::MAX_BYTES + 16 * 1024,
            ),
            FrameLimits::new(
                16 * 1024,
                1,
                crate::screenshot::MAX_BYTES,
                crate::screenshot::MAX_BYTES + 16 * 1024,
            ),
        ),
    ))
    .with_locator(LocatorInstallationProfile::new(
        CdpLocatorProductionProfile::new(ResourceScope::extension(120, 8), 8),
        pwright_codec_locator::LocatorCodecLimits::new(1024 * 1024),
        CapabilityWireProfile::new(
            ProtocolVersion::new(1, 0),
            FrameLimits::new(2 * 1024 * 1024, 0, 0, 2 * 1024 * 1024),
            FrameLimits::new(6 * 1024 * 1024, 0, 0, 6 * 1024 * 1024),
        ),
    ))
    .with_context_lifecycle(ContextLifecycleInstallationProfile::new(
        pwright_codec_context_lifecycle::ContextLifecycleCodecLimits::new(4096),
        CapabilityWireProfile::new(
            ProtocolVersion::new(1, 0),
            FrameLimits::new(16 * 1024, 0, 0, 16 * 1024),
            FrameLimits::new(16 * 1024, 0, 0, 16 * 1024),
        ),
    ))
    .with_context_creation(ContextCreationInstallationProfile::new(
        CdpContextCreationLimits::new(128),
        pwright_codec_context_creation::ContextCreationCodecLimits::new(4096),
        CapabilityWireProfile::new(
            ProtocolVersion::new(1, 0),
            FrameLimits::new(16 * 1024, 0, 0, 16 * 1024),
            FrameLimits::new(16 * 1024, 0, 0, 16 * 1024),
        ),
    ))
    .with_page_lifecycle(PageLifecycleInstallationProfile::new(
        CdpPageCloseLimits::new(128, 20),
        pwright_codec_page_lifecycle::PageLifecycleCodecLimits::new(4096),
        CapabilityWireProfile::new(
            ProtocolVersion::new(1, 0),
            FrameLimits::new(16 * 1024, 0, 0, 16 * 1024),
            FrameLimits::new(16 * 1024, 0, 0, 16 * 1024),
        ),
    ))
    .with_page_creation(PageCreationInstallationProfile::new(
        CdpPageCreationLimits::new(128),
        pwright_codec_page_creation::PageCreationCodecLimits::new(4096),
        CapabilityWireProfile::new(
            ProtocolVersion::new(1, 0),
            FrameLimits::new(16 * 1024, 0, 0, 16 * 1024),
            FrameLimits::new(16 * 1024, 0, 0, 16 * 1024),
        ),
    ))
    .with_navigation(NavigationInstallationProfile::new(
        CdpNavigationProductionProfile::new(
            ResourceScope::extension(120, 5),
            CdpNavigationLimits::new(16 * 1024, 256, 256, 128, 64 * 1024, 4096),
            8,
        ),
        pwright_codec_navigation::NavigationCodecLimits::new(16 * 1024, 128, 64 * 1024),
        CapabilityWireProfile::new(
            ProtocolVersion::new(1, 0),
            FrameLimits::new(256 * 1024, 0, 0, 256 * 1024),
            FrameLimits::new(256 * 1024, 0, 0, 256 * 1024),
        ),
    ))
    .with_dom_content(DomContentInstallationProfile::new(
        CdpDomContentProductionProfile::new(ResourceScope::extension(120, 6), 1024 * 1024, 8)
            .with_frame_concurrency(NonZeroUsize::new(8).expect("fixed positive concurrency")),
        pwright_codec_dom_content::DomContentCodecLimits::new(1024 * 1024, 4096),
        CapabilityWireProfile::new(
            ProtocolVersion::new(1, 0),
            FrameLimits::new(16 * 1024, 1, 1024 * 1024, 2 * 1024 * 1024),
            FrameLimits::new(16 * 1024, 1, 1024 * 1024, 2 * 1024 * 1024),
        ),
    ))
    .with_evaluation(EvaluationInstallationProfile::new(
        CdpEvaluationProductionProfile::new(
            ResourceScope::extension(120, 7),
            CdpEvaluationLimits::new(
                evaluation::SOURCE_BYTES,
                evaluation::GENERATED_SCRIPT_BYTES,
                evaluation::ERROR_BYTES,
                evaluation::value_limits(),
            ),
            8,
        )
        .with_user_gesture(true),
        pwright_codec_evaluation::EvaluationCodecLimits::new(
            evaluation::SOURCE_BYTES,
            evaluation::ERROR_BYTES,
            evaluation::value_limits(),
        )
        .map_err(HostError::backend)?,
        CapabilityWireProfile::new(
            ProtocolVersion::new(1, 0),
            FrameLimits::new(2 * 1024 * 1024, 0, 0, 2 * 1024 * 1024),
            FrameLimits::new(2 * 1024 * 1024, 0, 0, 2 * 1024 * 1024),
        ),
    ))
    .with_topology_events(events)
    .with_page_discovery(discovery)
    .with_extension_requirements(crate::locator_evaluation::Profile::new()?.requirements())
    .with_extension_requirements(crate::screenshot::requirements())
    .with_extension_requirements(crate::keyboard::requirements()?)
    .with_extension_requirements(crate::mouse::requirements()?)
    .with_extension_requirements(crate::input_start::requirements());
    let profile = if native_close {
        let confirmation = crate::close_services::native_profile()?;
        profile
            .with_extension_requirements(confirmation.requirements())
            .with_extension_event_subscribers(confirmation.event_subscribers())
            .map_err(HostError::backend)?
    } else {
        profile
    };
    #[cfg(feature = "concurrent-close")]
    let profile = if native_close {
        let close = crate::close_jobs::Profile::new(topology)?;
        profile
            .with_extension_requirements(close.requirements())
            .with_extension_event_subscribers(close.event_subscribers())
            .map_err(HostError::backend)?
    } else {
        profile
    };
    #[cfg(feature = "frame-lifecycle")]
    let profile = profile.with_extension_requirements(crate::frame_lifecycle::requirements()?);
    #[cfg(feature = "frame-navigation")]
    let profile = profile.with_extension_requirements(crate::frame_navigation::requirements()?);
    #[cfg(feature = "extra-headers")]
    let profile = profile.with_extension_requirements(crate::extra_headers::requirements()?);
    #[cfg(feature = "file-input")]
    let profile = profile.with_extension_requirements(crate::file_input::profile::requirements()?);
    #[cfg(feature = "handles")]
    let profile = crate::handles::select(profile)?;
    #[cfg(feature = "drag-runtime")]
    let profile = profile
        .with_extension_requirements(crate::drag::host::requirements()?)
        .with_extension_requirements(crate::locator::keyboard::requirements())
        .with_extension_event_subscribers(crate::drag::host::EVENT_SUBSCRIBERS)
        .map_err(HostError::backend)?;
    let profile = profile
        .with_derived_reclaiming_event_fanout()
        .map_err(HostError::backend)?;
    let requirements = profile.requirements();
    let mut limits = ResourceLimits::deny_all(requirements.scopes, requirements.leases);
    for class in ResourceClass::ALL {
        let bound = requirements.class(class);
        limits = limits.with_class(class, bound.global, bound.per_scope);
    }
    ChromiumAttachFactory::new(profile, limits, ObjectId::new(1)).map_err(HostError::backend)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(not(feature = "handles"))]
    #[test]
    fn ordinary_profile_does_not_install_retained_handles() {
        // Arrange
        let factory = attach_factory(Duration::from_secs(5)).unwrap();
        // Act
        let profile = factory.profile();
        // Assert
        assert!(profile.handle_lifecycle().is_none());
        assert!(profile.handle_evaluation().is_none());
        assert!(profile.handle_element_adoption().is_none());
    }

    #[test]
    fn native_close_selection_admits_one_additional_reclaiming_cursor() {
        // Arrange
        let ordinary = attach_factory(Duration::from_secs(5)).unwrap();
        let confirmation = crate::close_services::native_profile().unwrap();
        // Act
        let selected = attach_factory_with_native_close(Duration::from_secs(5), true).unwrap();
        let additional = confirmation.event_subscribers();
        #[cfg(feature = "concurrent-close")]
        let additional = additional
            + crate::close_jobs::Profile::new(selected.profile().topology())
                .unwrap()
                .event_subscribers();
        // Assert
        assert_eq!(
            selected
                .profile()
                .connection()
                .reclaiming_event_subscribers(),
            ordinary
                .profile()
                .connection()
                .reclaiming_event_subscribers()
                .map(|count| count + additional),
        );
        let bytes = confirmation
            .requirements()
            .class(ResourceClass::BackendStateBytes)
            .global;
        assert!(bytes >= 4096 + 16 * 1024);
        eprintln!("Native close declared state, owner and call bytes: {bytes}");
        assert_eq!(confirmation.event_subscribers(), 1);
        assert_eq!(
            selected
                .profile()
                .connection()
                .config()
                .pending_command_capacity,
            ordinary
                .profile()
                .connection()
                .config()
                .pending_command_capacity,
        );
    }

    #[cfg(feature = "concurrent-close")]
    #[test]
    fn native_lifecycle_attachments_need_no_control_outbox() {
        // Arrange
        let ordinary = attach_factory(Duration::from_secs(5)).unwrap();
        // Act
        let selected = attach_factory_with_native_close(Duration::from_secs(5), true).unwrap();
        // Assert
        assert_eq!(ordinary.profile().runtime().control_messages, None);
        assert_eq!(selected.profile().runtime().control_messages, None);
        assert_eq!(
            selected.profile().runtime().live_objects,
            crate::close_jobs::MAX_OBJECTS
        );
        assert_eq!(
            selected.profile().runtime().pending_calls,
            crate::close_jobs::MAX_CALLS
        );
    }

    #[test]
    fn connection_admits_full_mouse_batches_without_changing_write_backpressure() {
        // Arrange
        let factory = attach_factory(Duration::from_secs(5)).unwrap();
        // Act
        let config = factory.profile().connection().config();
        // Assert
        assert_eq!(crate::mouse::MAX_CLICK_COMMANDS, 65);
        assert_eq!(crate::mouse::MAX_PENDING_COMMANDS, 520);
        assert_eq!(config.pending_command_capacity, 552);
        assert_eq!(config.write_channel_capacity, 32);
        assert_eq!(config.max_command_bytes, 4 * 1024 * 1024);
    }
}
