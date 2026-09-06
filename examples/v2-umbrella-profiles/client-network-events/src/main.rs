use pwright::client_network_events::PageNetworkSubscriptionExt;

fn main() {
    let _ = std::any::type_name::<dyn PageNetworkSubscriptionExt>();
}
