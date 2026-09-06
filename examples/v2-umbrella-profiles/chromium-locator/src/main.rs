use pwright::chromium_locator::{
    CdpLocatorProductionProfile, ChromiumLocatorProfile, FrameLimits, LocatorCodecLimits,
    ObjectId, ResourceScope,
};

fn main() {
    let frames = FrameLimits::new(16 * 1024 * 1024, 0, 0, 16 * 1024 * 1024);
    let profile = ChromiumLocatorProfile::v2(
        CdpLocatorProductionProfile::new(ResourceScope::Object(ObjectId::new(7)), 1),
        LocatorCodecLimits::new(1024 * 1024),
        frames,
        frames,
    );
    let requirements = profile.requirements();
    println!(
        "Chromium Locator: {} resource scopes and {} leases",
        requirements.scopes, requirements.leases
    );
}
