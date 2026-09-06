use pwright_backend_cdp_handle_lifecycle::CdpHandleDisposeLimits;

fn main() {
    let limits = CdpHandleDisposeLimits::new(4096);
    println!(
        "handle-lifecycle-only profile: {} native ID bytes",
        limits.native_id_bytes
    );
}
