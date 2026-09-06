use pwright_backend_cdp_page_lifecycle::CdpPageCloseLimits;

fn main() {
    let limits = CdpPageCloseLimits::new(256, 20);
    println!(
        "page-lifecycle-only profile: {} native ID bytes, {} confirmation attempts",
        limits.native_id_bytes, limits.confirmation_attempts
    );
}
