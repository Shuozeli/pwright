use pwright_process_metrics::{ProcessId, ProcessMetricsLimits, ProcessMetricsProbe};

fn main() {
    if let Err(error) = run() {
        eprintln!("pwright-process-metrics: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut arguments = std::env::args_os();
    let _program = arguments.next();
    let browser = arguments
        .next()
        .ok_or("usage: pwright-process-metrics <browser-pid> [runtime-pid]")?;
    let runtime = arguments.next();
    if arguments.next().is_some() {
        return Err("usage: pwright-process-metrics <browser-pid> [runtime-pid]".into());
    }
    let browser = parse_process_id(browser)?;
    let runtime = runtime.map(parse_process_id).transpose()?;
    let limits = ProcessMetricsLimits::new(8 * 1024 * 1024, 16_384, 64 * 1024)?;
    let probe = ProcessMetricsProbe::system(limits);
    let metrics = match runtime {
        Some(runtime) => probe.sample(runtime, browser)?,
        None => probe.sample_current_runtime(browser)?,
    };
    println!(
        "runtime\tpid={}\tprocesses=1\trss_bytes={}",
        metrics.runtime.process_id.get(),
        metrics.runtime.resident_bytes
    );
    println!(
        "browser_parent\tpid={}\tprocesses={}\trss_bytes={}",
        metrics.browser.process_id.get(),
        metrics.browser.browser_parent.processes,
        metrics.browser.browser_parent.resident_bytes
    );
    for (name, role) in [
        ("renderer", metrics.browser.renderers),
        ("gpu", metrics.browser.gpu),
        ("utility", metrics.browser.utility),
        ("other_child", metrics.browser.other_children),
        ("browser_total_rss_sum", metrics.browser.total),
    ] {
        println!(
            "{name}\tprocesses={}\trss_bytes={}",
            role.processes, role.resident_bytes
        );
    }
    Ok(())
}

fn parse_process_id(value: std::ffi::OsString) -> Result<ProcessId, Box<dyn std::error::Error>> {
    let value = value.into_string().map_err(|_| "PID must be UTF-8")?;
    Ok(ProcessId::new(value.parse()?)?)
}
