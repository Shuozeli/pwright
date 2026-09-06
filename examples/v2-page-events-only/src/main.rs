use pwright_backend_cdp_page_events::CdpPageEventAdapter;

fn main() {
    println!(
        "page-events-only profile: {} adapter bytes",
        std::mem::size_of::<CdpPageEventAdapter>()
    );
}
