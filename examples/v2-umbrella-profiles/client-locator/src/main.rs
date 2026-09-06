use pwright::client_locator::LocatorCountExt;
use pwright::locator::CssSelector;

fn main() {
    let selector = CssSelector::new("button").expect("static selector should be valid");
    println!(
        "{}: {selector:?}",
        std::any::type_name::<dyn LocatorCountExt>()
    );
}
