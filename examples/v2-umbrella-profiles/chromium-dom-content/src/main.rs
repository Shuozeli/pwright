use pwright::chromium_dom_content::{
    CdpDomContentProductionProfile, ChromiumDomContentProfile, DomContentCodecLimits, FrameLimits,
    ObjectId, ProtocolVersion, ResourceScope,
};

fn main() {
    let profile = ChromiumDomContentProfile::new(
        CdpDomContentProductionProfile::new(
            ResourceScope::Object(ObjectId::new(1)),
            16 * 1024,
            4,
        )
        .with_context_routes(1, 256),
        DomContentCodecLimits::new(16 * 1024, 1_024),
        ProtocolVersion::new(1, 0),
        FrameLimits::new(4_096, 1, 16 * 1024, 20 * 1024),
        FrameLimits::new(4_096, 1, 16 * 1024, 20 * 1024),
    );

    let requirements = profile.requirements();
    println!(
        "Chromium DOM content: {} resource scopes and {} leases",
        requirements.scopes, requirements.leases
    );
}
