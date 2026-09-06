use pwright_backend_cdp_locator::CdpLocatorService;

fn main() {
    println!(
        "locator-only profile: {}-byte CDP Locator service handle",
        std::mem::size_of::<CdpLocatorService>()
    );
}
