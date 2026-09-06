use pwright::client_navigation::PageGotoExt;
use pwright::navigation::{GotoParams, WaitUntil};

fn main() {
    let request = GotoParams::new("https://example.com").with_wait_until(WaitUntil::Load);
    let extension_is_installed = std::any::type_name::<dyn PageGotoExt>();
    println!("{}: {}", extension_is_installed, request.url);
}
