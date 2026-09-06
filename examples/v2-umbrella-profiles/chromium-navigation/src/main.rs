use pwright::chromium_navigation::{
    CdpNavigationLimits, CdpNavigationProductionProfile, ChromiumNavigationProfile, FrameLimits,
    NavigationCodecLimits, ObjectId, ProtocolVersion, ResourceScope,
};

fn main() {
    let profile = ChromiumNavigationProfile::new(
        CdpNavigationProductionProfile::new(
            ResourceScope::Object(ObjectId::new(1)),
            CdpNavigationLimits::new(4_096, 256, 64, 64, 16_384, 1_024),
            4,
        ),
        NavigationCodecLimits::new(16_384, 64, 16_384),
        ProtocolVersion::new(1, 0),
        FrameLimits::new(65_536, 0, 0, 65_536),
        FrameLimits::new(65_536, 0, 0, 65_536),
    );

    let requirements = profile.requirements();
    println!(
        "chromium navigation: {} resource scopes and {} leases",
        requirements.scopes, requirements.leases
    );
}
