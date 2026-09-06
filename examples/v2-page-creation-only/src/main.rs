use pwright_backend_cdp_page_creation::CdpPageCreationLimits;

fn main() {
    let limits = CdpPageCreationLimits::new(256).with_cleanup_confirmation_attempts(20);
    println!(
        "page-creation-only profile: {} native ID bytes, {} cleanup attempts",
        limits.native_id_bytes, limits.cleanup_confirmation_attempts
    );
}
