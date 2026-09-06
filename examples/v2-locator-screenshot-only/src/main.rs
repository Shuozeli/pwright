use pwright_backend_cdp_selector_geometry::CdpSelectorGeometryProductionProfile;
use pwright_composition_chromium_locator_screenshot::{
    CdpScreenshotProductionProfile, ChromiumLocatorScreenshotProfile, FrameLimits,
    LocatorScreenshotCodecLimits, ProtocolVersion, ResourceScope,
};

fn main() {
    let profile = ChromiumLocatorScreenshotProfile::new(
        CdpScreenshotProductionProfile::new(4 * 1024 * 1024, 1),
        LocatorScreenshotCodecLimits::new(4 * 1024 * 1024, 64 * 1024, 32, 256),
        ProtocolVersion::new(1, 0),
        FrameLimits::new(4_096, 1, 4 * 1024 * 1024, 4 * 1024 * 1024 + 4_096),
        FrameLimits::new(4_096, 1, 4 * 1024 * 1024, 4 * 1024 * 1024 + 4_096),
    );

    let selector = CdpSelectorGeometryProductionProfile::new(ResourceScope::Connection, 1)
        .with_context_routes(8, 256);
    let requirements = profile
        .requirements()
        .combine_concurrent(selector.requirements());
    println!(
        "Locator-screenshot plus first-party selector profile: {} resource scopes and {} leases",
        requirements.scopes, requirements.leases
    );
}
