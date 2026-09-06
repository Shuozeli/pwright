use pwright::chromium_locator_screenshot::{
    CdpScreenshotProductionProfile, ChromiumLocatorScreenshotProfile, FrameLimits,
    LocatorScreenshotCodecLimits, ProtocolVersion,
};

fn main() {
    let profile = ChromiumLocatorScreenshotProfile::new(
        CdpScreenshotProductionProfile::new(4 * 1024 * 1024, 1),
        LocatorScreenshotCodecLimits::new(4 * 1024 * 1024, 64 * 1024, 32, 256),
        ProtocolVersion::new(1, 0),
        FrameLimits::new(4_096, 1, 4 * 1024 * 1024, 4 * 1024 * 1024 + 4_096),
        FrameLimits::new(4_096, 1, 4 * 1024 * 1024, 4 * 1024 * 1024 + 4_096),
    );

    let requirements = profile.requirements();
    println!(
        "Chromium Locator screenshot: {} resource scopes and {} leases",
        requirements.scopes, requirements.leases
    );
}
