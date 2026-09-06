use pwright_backend_cdp_topology::CdpTopologyLimits;

fn main() {
    let limits = CdpTopologyLimits::new(64, 8, 16, 64, 64, 256);
    let maximum_retained_native_bytes = limits
        .maximum_retained_native_bytes()
        .expect("topology-only limits should fit");

    println!(
        "topology-only profile: {} maximum retained native bytes",
        maximum_retained_native_bytes
    );
}
