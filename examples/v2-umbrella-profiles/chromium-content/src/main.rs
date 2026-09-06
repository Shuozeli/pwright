use pwright::chromium_content::{
    CdpScreenshotProductionProfile, ChromiumContentProfile, ContentCodecLimits, FrameLimits,
    ProtocolVersion,
};

fn main() {
    let profile = ChromiumContentProfile::new(
        CdpScreenshotProductionProfile::new(4 * 1024 * 1024, 1),
        ContentCodecLimits::new(4 * 1024 * 1024, 1_024),
        ProtocolVersion::new(1, 0),
        FrameLimits::new(4_096, 1, 4 * 1024 * 1024, 4 * 1024 * 1024 + 4_096),
        FrameLimits::new(4_096, 1, 4 * 1024 * 1024, 4 * 1024 * 1024 + 4_096),
    );

    let requirements = profile.requirements();
    println!(
        "Chromium Page screenshot: {} resource scopes and {} leases",
        requirements.scopes, requirements.leases
    );
}
