use pwright::chromium_network_events::ChromiumNetworkEventDeliveryProfile;

fn main() {
    println!(
        "{}",
        std::mem::size_of::<ChromiumNetworkEventDeliveryProfile>()
    );
}
