use pwright_backend_cdp_handle_evaluation::CdpHandleEvaluationLimits;

fn main() {
    let limits = CdpHandleEvaluationLimits::default();
    println!(
        "handle-evaluation-only profile: {} native ID bytes, {} preview bytes",
        limits.native_id_bytes(),
        limits.preview_bytes()
    );
}
