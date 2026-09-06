use pwright_backend_cdp_context_lifecycle::CdpContextCloseLimits;

fn main() {
    let limits = CdpContextCloseLimits::new(256);
    println!(
        "context-lifecycle-only profile: {} native ID bytes",
        limits.native_id_bytes
    );
}
