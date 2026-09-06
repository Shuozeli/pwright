use pwright_backend_cdp_network_events::CdpNetworkEventBatch;

fn main() {
    let batch = CdpNetworkEventBatch::default();
    println!("network-events-only profile: empty={}", batch.is_empty());
}
