use pwright::chromium_evaluation::{
    CdpEvaluationLimits, CdpEvaluationProductionProfile, ChromiumEvaluationProfile,
    EvaluationCodecLimits, EvaluationValueLimits, FrameLimits, ObjectId, ProtocolVersion,
    ResourceScope,
};

fn main() {
    let values = EvaluationValueLimits::new(8, 256, 256, 16_384, 16_384)
        .expect("evaluation limits should be valid");
    let profile = ChromiumEvaluationProfile::new(
        CdpEvaluationProductionProfile::new(
            ResourceScope::Object(ObjectId::new(1)),
            CdpEvaluationLimits::new(4_096, 64 * 1024, 1_024, values),
            4,
        ),
        EvaluationCodecLimits::new(4_096, 1_024, values)
            .expect("evaluation codec limits should be valid"),
        ProtocolVersion::new(1, 0),
        FrameLimits::new(65_536, 0, 0, 65_536),
        FrameLimits::new(65_536, 0, 0, 65_536),
    );

    let requirements = profile.requirements();
    println!(
        "chromium evaluation: {} resource scopes and {} leases",
        requirements.scopes, requirements.leases
    );
}
