use pwright_backend_cdp_context_creation::CdpContextCreationLimits;

fn main() {
    let limits = CdpContextCreationLimits::new(256);
    println!(
        "context-creation-only profile: {} native ID bytes",
        limits.native_id_bytes
    );
}
