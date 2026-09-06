use pwright::client_page_events::PageLoadSubscriptionExt;

fn main() {
    let _ = std::any::type_name::<dyn PageLoadSubscriptionExt>();
}
