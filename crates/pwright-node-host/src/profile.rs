use std::num::NonZeroUsize;
use std::time::Duration;

#[cfg(feature = "network-events")]
use pwright_backend_cdp::CdpNetworkEventProductionProfile;
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
#[cfg(feature = "network-events")]
use pwright_composition_chromium::ChromiumNetworkEventsProductionProfile;
#[cfg(feature = "locator-screenshot")]
use pwright_composition_chromium::LocatorScreenshotInstallationProfile;
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
#[cfg(feature = "network-events")]
use pwright_network_model::NetworkLifecycleLimits;
use pwright_runtime::RuntimeLimits;
use pwright_server_wire::ServerSupervisorLimits;
use pwright_transport::FrameLimits;

use crate::{HostError, evaluation};

const MAX_TOPOLOGY_OBJECTS: usize = 64;
const BASE_RUNTIME_LIVE_OBJECTS: usize = 256;
/// One bounded connection replay window large enough for the declared
/// 16-Page/128-session/256-world authoritative reattach burst and the small
/// historical console events that `Runtime.enable` can replay while the
/// caller-driven topology cursor is still initializing existing Pages.
const MAX_CONNECTION_EVENT_BACKLOG: usize = 8192;
/// Raising the item bound must not multiply the selected single-event maximum
/// into an unbounded allocation plan. Retained event payloads keep a fixed
/// 512 MiB aggregate ceiling; exact byte leases can therefore fail earlier
/// than the item bound when unusually large events dominate the queue.
const MAX_CONNECTION_RETAINED_EVENT_BYTES: u64 = 512 * 1024 * 1024;
#[cfg(feature = "large-evaluation-inputs")]
const MAX_CDP_COMMAND_BYTES: usize = 8 * 1024 * 1024;
#[cfg(not(feature = "large-evaluation-inputs"))]
const MAX_CDP_COMMAND_BYTES: usize = 4 * 1024 * 1024;
#[cfg(feature = "large-route-post-data")]
// A 3 MiB decoded POST may carry a 4 MiB base64 postDataEntries copy as
// well as an unescaped plain postData copy. Leave finite JSON-envelope room;
// pathological escaping is still rejected at this independent transport cap.
const MAX_CDP_EVENT_BYTES: usize = 8 * 1024 * 1024;
#[cfg(not(feature = "large-route-post-data"))]
const MAX_CDP_EVENT_BYTES: usize = 256 * 1024;
#[cfg(any(feature = "large-evaluation-inputs", feature = "large-route-post-data"))]
const MAX_CDP_WEBSOCKET_BYTES: usize = 8 * 1024 * 1024;
#[cfg(not(any(feature = "large-evaluation-inputs", feature = "large-route-post-data")))]
const MAX_CDP_WEBSOCKET_BYTES: usize = 4 * 1024 * 1024;
#[cfg(feature = "network-events")]
const MAX_NETWORK_ACTIVE_REQUESTS_PER_PAGE: usize = 64;
#[cfg(feature = "network-events")]
const MAX_NETWORK_PAGE_LANES: usize = 8;
#[cfg(feature = "network-events")]
const MAX_NETWORK_POST_DATA_BYTES: usize = 256 * 1024;
#[cfg(all(feature = "large-route-post-data", feature = "route-request-history"))]
const MAX_NETWORK_LARGE_POST_DATA_BYTES: usize = 3 * 1024 * 1024;
#[cfg(all(feature = "large-route-post-data", feature = "route-request-history"))]
const MAX_NETWORK_LARGE_POST_DATA_REQUESTS_PER_PAGE: usize = 4;

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
            event_channel_capacity: MAX_CONNECTION_EVENT_BACKLOG,
            // Preserve no-delay Mouse batch initiation for every admitted native
            // operation, plus the existing non-Mouse allowance. Write queue
            // backpressure is unchanged; no serializing click scheduler is added.
            pending_command_capacity: 32 + crate::mouse::MAX_PENDING_COMMANDS,
            max_command_bytes: MAX_CDP_COMMAND_BYTES,
            max_response_bytes: 4 * 1024 * 1024,
            max_event_bytes: MAX_CDP_EVENT_BYTES,
            max_websocket_bytes: MAX_CDP_WEBSOCKET_BYTES,
            max_protocol_error_bytes: 4096,
        },
    )
    .with_retained_event_byte_capacity(MAX_CONNECTION_RETAINED_EVENT_BYTES)
    .map_err(HostError::backend)?
    .with_pending_session_retirement(256)
    .map_err(HostError::backend)?;
    // A public browser-root session must receive raw events from its own
    // native session, including worlds outside Pwright's frame inventory.
    #[cfg(not(feature = "browser-cdp-session"))]
    let connection = connection
        // Ordinary profiles install only main/Pwright utility-world consumers.
        .with_event_admission_policy(
            pwright_composition_chromium::chromium_frame_world_event_admission(),
        )
        .map_err(HostError::backend)?;
    let topology = CdpTopologyProductionProfile::new(
        CdpTopologyLimits::new(MAX_TOPOLOGY_OBJECTS, 16, 16, 64, 128, 256),
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
    let runtime_live_objects = BASE_RUNTIME_LIVE_OBJECTS;
    #[cfg(feature = "network-events")]
    let runtime_live_objects = runtime_live_objects.max(
        MAX_TOPOLOGY_OBJECTS + (2 * MAX_NETWORK_ACTIVE_REQUESTS_PER_PAGE * MAX_NETWORK_PAGE_LANES),
    );
    let runtime_limits = RuntimeLimits::new(runtime_live_objects, 16).with_active_commands(8);
    #[cfg(feature = "concurrent-close")]
    let runtime_limits = if native_close {
        RuntimeLimits::new(
            runtime_live_objects.max(crate::close_jobs::MAX_OBJECTS),
            16usize.max(crate::close_jobs::MAX_CALLS),
        )
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
            crate::screenshot::MAX_CONCURRENT_CAPTURES,
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
    ));
    #[cfg(feature = "locator-screenshot")]
    let profile = profile.with_locator_screenshot(LocatorScreenshotInstallationProfile::new(
        CdpLocatorProductionProfile::new(ResourceScope::extension(120, 18), 8),
        pwright_backend_cdp_content::CdpScreenshotProductionProfile::new(
            crate::screenshot::MAX_BYTES,
            crate::screenshot::MAX_CONCURRENT_CAPTURES,
        ),
        pwright_codec_locator_screenshot::LocatorScreenshotCodecLimits::new(
            crate::screenshot::MAX_BYTES,
            1024 * 1024,
            32,
            64,
        ),
        CapabilityWireProfile::new(
            ProtocolVersion::new(1, 0),
            FrameLimits::new(
                1024 * 1024 + 16 * 1024,
                1,
                crate::screenshot::MAX_BYTES,
                crate::screenshot::MAX_BYTES + 1024 * 1024 + 16 * 1024,
            ),
            FrameLimits::new(
                16 * 1024,
                1,
                crate::screenshot::MAX_BYTES,
                crate::screenshot::MAX_BYTES + 16 * 1024,
            ),
        ),
    ));
    let evaluation_limits = CdpEvaluationLimits::new(
        evaluation::SOURCE_BYTES,
        evaluation::INPUT_GENERATED_SCRIPT_BYTES,
        evaluation::ERROR_BYTES,
        evaluation::value_limits(),
    )
    .with_input_string_bytes(evaluation::INPUT_STRING_BYTES);
    #[cfg(feature = "large-evaluation-results")]
    let evaluation_limits = evaluation_limits
        .with_chunked_string_results(evaluation::STRING_BYTES, evaluation::CHUNK_UTF16_UNITS)
        .map_err(HostError::backend)?;
    #[cfg(feature = "large-evaluation-results")]
    let evaluation_wire = FrameLimits::new(
        evaluation::RESULT_WIRE_BYTES,
        0,
        0,
        evaluation::RESULT_WIRE_BYTES,
    );
    #[cfg(not(feature = "large-evaluation-results"))]
    let evaluation_wire = {
        #[cfg(feature = "large-evaluation-inputs")]
        let bytes = 4 * 1024 * 1024;
        #[cfg(not(feature = "large-evaluation-inputs"))]
        let bytes = 2 * 1024 * 1024;
        FrameLimits::new(bytes, 0, 0, bytes)
    };
    let profile = profile
        .with_evaluation(EvaluationInstallationProfile::new(
            CdpEvaluationProductionProfile::new(
                ResourceScope::extension(120, 7),
                evaluation_limits,
                8,
            )
            .with_user_gesture(true),
            pwright_codec_evaluation::EvaluationCodecLimits::new(
                evaluation::SOURCE_BYTES,
                evaluation::ERROR_BYTES,
                evaluation::value_limits(),
            )
            .and_then(|limits| limits.with_input_value_limits(evaluation::input_value_limits()))
            .map_err(HostError::backend)?,
            CapabilityWireProfile::new(
                ProtocolVersion::new(1, 0),
                evaluation_wire,
                evaluation_wire,
            ),
        ))
        .with_topology_events(events)
        .with_page_discovery(discovery)
        .with_extension_requirements(crate::locator_evaluation::Profile::new()?.requirements())
        .with_extension_requirements(crate::screenshot::requirements())
        .with_extension_requirements(crate::keyboard::requirements()?)
        .with_extension_requirements(crate::mouse::requirements()?)
        .with_extension_requirements(crate::input_start::requirements());
    #[cfg(feature = "touchscreen-input")]
    let profile = profile.with_extension_requirements(crate::touchscreen::requirements()?);
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
    #[cfg(feature = "url-wait")]
    let profile = profile.with_extension_requirements(crate::url_wait::requirements());
    #[cfg(feature = "set-content")]
    let profile = profile.with_extension_requirements(crate::set_content::requirements());
    #[cfg(feature = "page-bindings")]
    let profile = profile.with_extension_requirements(crate::page_bindings::requirements());
    #[cfg(feature = "page-routes")]
    let profile = profile.with_extension_requirements(crate::page_routes::requirements());
    #[cfg(feature = "page-dialogs")]
    let profile = profile.with_extension_requirements(crate::page_dialogs::requirements());
    #[cfg(feature = "download-events")]
    let profile = profile.with_extension_requirements(crate::download_events::requirements());
    #[cfg(feature = "worker-events")]
    let profile = profile.with_extension_requirements(crate::worker_events::requirements());
    #[cfg(feature = "worker-handle-evaluation")]
    let profile = profile.with_extension_requirements(crate::worker_handles::requirements());
    #[cfg(feature = "worker-console-events")]
    let profile = profile.with_extension_requirements(crate::worker_console_events::requirements());
    #[cfg(feature = "file-chooser-events")]
    let profile = profile.with_extension_requirements(crate::page_file_chooser::requirements());
    #[cfg(feature = "page-errors")]
    let profile = profile.with_extension_requirements(crate::page_errors::requirements());
    #[cfg(feature = "page-pdf")]
    let profile = profile.with_extension_requirements(crate::page_pdf::requirements());
    #[cfg(feature = "console-history")]
    let profile = profile.with_extension_requirements(crate::console_events::requirements());
    #[cfg(feature = "init-scripts")]
    let profile = profile.with_extension_requirements(crate::init_scripts::requirements());
    #[cfg(feature = "wait-for-function")]
    let profile = profile.with_extension_requirements(crate::wait_for_function::requirements());
    #[cfg(feature = "wait-for-timeout")]
    let profile = profile.with_extension_requirements(crate::wait_for_timeout::requirements());
    #[cfg(feature = "emulate-media")]
    let profile = profile.with_extension_requirements(crate::emulate_media::requirements());
    #[cfg(feature = "bypass-csp")]
    let profile = profile.with_extension_requirements(crate::bypass_csp::requirements());
    #[cfg(feature = "javascript-disabled")]
    let profile = profile.with_extension_requirements(crate::javascript_disabled::requirements());
    #[cfg(feature = "context-timezone")]
    let profile = profile.with_extension_requirements(crate::context_timezone::requirements());
    #[cfg(feature = "context-offline")]
    let profile = profile.with_extension_requirements(crate::context_offline::requirements());
    #[cfg(feature = "context-geolocation")]
    let profile = profile.with_extension_requirements(crate::context_geolocation::requirements());
    #[cfg(feature = "context-geolocation-permission")]
    let profile =
        profile.with_extension_requirements(crate::context_geolocation_permission::requirements());
    #[cfg(feature = "context-http-credentials")]
    let profile =
        profile.with_extension_requirements(crate::context_http_credentials::requirements());
    #[cfg(feature = "context-proxy")]
    let profile = profile.with_extension_requirements(crate::context_proxy::requirements());
    #[cfg(any(feature = "page-routes", feature = "context-http-credentials"))]
    let profile = profile
        .with_extension_event_subscribers(1)
        .map_err(HostError::backend)?;
    #[cfg(feature = "timeout-settings")]
    let profile = profile.with_extension_requirements(crate::timeout_settings::requirements());
    #[cfg(feature = "response-body")]
    let profile = profile.with_extension_requirements(crate::response_body::requirements());
    #[cfg(feature = "network-events")]
    let profile = {
        let lifecycle = NetworkLifecycleLimits::new(
            MAX_NETWORK_ACTIVE_REQUESTS_PER_PAGE,
            256,
            64 * 1024,
            32,
            64,
            128,
            64 * 1024,
            MAX_NETWORK_POST_DATA_BYTES,
            4 * 1024,
            4 * 1024,
        );
        #[cfg(all(feature = "large-route-post-data", feature = "route-request-history"))]
        let lifecycle = NetworkLifecycleLimits {
            post_data_bytes: MAX_NETWORK_LARGE_POST_DATA_BYTES,
            ..lifecycle
        };
        #[cfg(feature = "worker-network-events")]
        let lifecycle =
            lifecycle.with_native_scope_bytes(crate::worker_events::MAX_WORKER_ID_BYTES);
        #[cfg(all(feature = "large-route-post-data", feature = "route-request-history"))]
        let adapter = CdpNetworkEventProductionProfile::with_bounded_large_post_data(
            lifecycle,
            ResourceScope::extension(135, 2),
            MAX_NETWORK_POST_DATA_BYTES,
            MAX_NETWORK_LARGE_POST_DATA_REQUESTS_PER_PAGE,
        )
        .map_err(HostError::backend)?;
        #[cfg(not(all(feature = "large-route-post-data", feature = "route-request-history")))]
        let adapter =
            CdpNetworkEventProductionProfile::new(lifecycle, ResourceScope::extension(135, 2))
                .map_err(HostError::backend)?;
        #[cfg(not(feature = "worker-network-events"))]
        let network = ChromiumNetworkEventsProductionProfile::new_with_context_limit(
            adapter,
            MAX_NETWORK_PAGE_LANES,
            MAX_NETWORK_PAGE_LANES,
        )
        .map_err(HostError::backend)?;
        #[cfg(feature = "worker-network-events")]
        let network =
            ChromiumNetworkEventsProductionProfile::new_with_context_and_related_session_limit(
                adapter,
                MAX_NETWORK_PAGE_LANES,
                MAX_NETWORK_PAGE_LANES,
                crate::worker_events::MAX_WORKERS,
            )
            .map_err(HostError::backend)?;
        profile
            .with_network_events(network)
            .with_extension_requirements(crate::network_events::requirements())
    };
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
    #[cfg(feature = "cdp-session")]
    let profile = profile
        .with_extension_requirements(crate::cdp_sessions::requirements())
        .with_extension_event_subscribers(1)
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

    #[cfg(feature = "network-events")]
    #[test]
    fn network_profile_admits_a_bounded_parallel_page_load() {
        // Arrange
        let factory = attach_factory(Duration::from_secs(5)).unwrap();

        // Act
        let network = factory
            .profile()
            .network_events()
            .expect("network events should be selected");

        // Assert
        assert_eq!(
            network.adapter().limits().active_requests,
            MAX_NETWORK_ACTIVE_REQUESTS_PER_PAGE
        );
        assert_eq!(
            network.adapter().ordinary_post_data_bytes(),
            MAX_NETWORK_POST_DATA_BYTES
        );
        #[cfg(not(all(feature = "large-route-post-data", feature = "route-request-history")))]
        {
            assert_eq!(
                network.adapter().limits().post_data_bytes,
                MAX_NETWORK_POST_DATA_BYTES
            );
            assert_eq!(network.adapter().large_post_data_requests(), 0);
        }
        #[cfg(all(feature = "large-route-post-data", feature = "route-request-history"))]
        {
            assert_eq!(
                network.adapter().limits().post_data_bytes,
                MAX_NETWORK_LARGE_POST_DATA_BYTES
            );
            assert_eq!(
                network.adapter().large_post_data_requests(),
                MAX_NETWORK_LARGE_POST_DATA_REQUESTS_PER_PAGE
            );
        }
        assert_eq!(network.max_page_lanes(), MAX_NETWORK_PAGE_LANES);
        assert_eq!(network.max_browser_contexts(), MAX_NETWORK_PAGE_LANES);
        #[cfg(not(feature = "worker-network-events"))]
        assert_eq!(network.max_related_sessions(), 0);
        #[cfg(feature = "worker-network-events")]
        assert_eq!(
            network.max_related_sessions(),
            crate::worker_events::MAX_WORKERS
        );
        assert_eq!(
            factory.profile().runtime().live_objects,
            MAX_TOPOLOGY_OBJECTS
                + (2 * MAX_NETWORK_ACTIVE_REQUESTS_PER_PAGE * MAX_NETWORK_PAGE_LANES)
        );
        assert!(network.adapter().maximum_retained_bytes() > 0);
    }

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

    #[cfg(any(feature = "page-routes", feature = "context-http-credentials"))]
    #[test]
    fn request_interception_profiles_share_one_reclaiming_cursor() {
        // Arrange
        let factory = attach_factory(Duration::from_secs(5)).unwrap();
        let profile = factory.profile();
        let built_in_subscribers = usize::from(profile.topology_events().is_some())
            + usize::from(profile.page_events())
            + profile
                .network_events()
                .map_or(0, |network| network.max_page_lanes())
            + profile.navigation().map_or(0, |navigation| {
                navigation.backend().max_concurrent_operations()
            })
            + profile
                .locator()
                .map_or(0, |locator| locator.backend().max_concurrent_actions());
        #[cfg(feature = "drag-and-drop")]
        let other_extension_subscribers = crate::drag::host::EVENT_SUBSCRIBERS;
        #[cfg(not(feature = "drag-and-drop"))]
        let other_extension_subscribers = 0;

        // Act
        let required = profile.required_reclaiming_event_subscribers();

        // Assert
        assert_eq!(
            required,
            built_in_subscribers + other_extension_subscribers + 1
        );
        assert_eq!(
            profile.connection().reclaiming_event_subscribers(),
            Some(required)
        );
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
        let expected_live_objects = crate::close_jobs::MAX_OBJECTS;
        #[cfg(feature = "network-events")]
        let expected_live_objects = expected_live_objects.max(
            MAX_TOPOLOGY_OBJECTS
                + (2 * MAX_NETWORK_ACTIVE_REQUESTS_PER_PAGE * MAX_NETWORK_PAGE_LANES),
        );
        // Act
        let selected = attach_factory_with_native_close(Duration::from_secs(5), true).unwrap();
        // Assert
        assert_eq!(ordinary.profile().runtime().control_messages, None);
        assert_eq!(selected.profile().runtime().control_messages, None);
        assert_eq!(
            selected.profile().runtime().live_objects,
            expected_live_objects
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
        assert_eq!(config.event_channel_capacity, MAX_CONNECTION_EVENT_BACKLOG);
        assert_eq!(
            factory.profile().connection().retained_event_bytes(),
            MAX_CONNECTION_RETAINED_EVENT_BYTES
        );
        assert_eq!(
            factory
                .profile()
                .connection()
                .requirements()
                .class(ResourceClass::EventBytes)
                .global,
            MAX_CONNECTION_RETAINED_EVENT_BYTES + config.max_event_bytes as u64
        );
        assert_eq!(config.max_command_bytes, MAX_CDP_COMMAND_BYTES);
        assert_eq!(config.max_event_bytes, MAX_CDP_EVENT_BYTES);
        assert_eq!(config.max_websocket_bytes, MAX_CDP_WEBSOCKET_BYTES);
        #[cfg(not(feature = "large-route-post-data"))]
        assert_eq!(config.max_event_bytes, 256 * 1024);
        #[cfg(feature = "large-route-post-data")]
        {
            assert_eq!(config.max_event_bytes, 8 * 1024 * 1024);
            assert_eq!(config.max_websocket_bytes, config.max_event_bytes);
            // The unescaped 3 MiB body plus its base64 copy leave finite
            // headroom for ordinary CDP metadata at this transport bound.
            let body_bytes: usize = 3 * 1024 * 1024;
            let base64_bytes = body_bytes.div_ceil(3) * 4;
            assert!(body_bytes + base64_bytes + 64 * 1024 < config.max_event_bytes);
        }
    }

    #[cfg(feature = "network-events")]
    #[test]
    fn large_route_body_does_not_expand_network_history_body_budget() {
        // Arrange
        #[cfg(feature = "large-route-post-data")]
        let captured_route_limit = crate::page_routes::MAX_CAPTURED_POST_DATA_BYTES;

        // Act
        let history_limit = MAX_NETWORK_POST_DATA_BYTES;

        // Assert
        assert_eq!(history_limit, 256 * 1024);
        #[cfg(feature = "large-route-post-data")]
        assert!(captured_route_limit > history_limit);
        #[cfg(all(feature = "large-route-post-data", feature = "route-request-history"))]
        {
            assert_eq!(MAX_NETWORK_LARGE_POST_DATA_BYTES, captured_route_limit);
            assert_eq!(MAX_NETWORK_LARGE_POST_DATA_REQUESTS_PER_PAGE, 4);
        }
    }
}
