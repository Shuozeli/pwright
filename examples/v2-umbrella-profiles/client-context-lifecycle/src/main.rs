use pwright::client_context_lifecycle::BrowserContextCloseExt;

fn main() {
    println!(
        "{}",
        std::any::type_name::<dyn BrowserContextCloseExt>()
    );
}
