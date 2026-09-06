use pwright_backend_cdp_input::CdpInputCoordinator;

fn main() {
    let coordinator = CdpInputCoordinator::new();
    let shared = coordinator.clone();

    assert!(coordinator.shares_state_with(&shared));
    println!("input-only profile: one shared Page input coordinator");
}
