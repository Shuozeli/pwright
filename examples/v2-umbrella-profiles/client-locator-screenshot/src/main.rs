use pwright::client_locator_screenshot::LocatorScreenshotExt;
use pwright::locator_screenshot::ScreenshotOptions;

fn main() {
    let options = ScreenshotOptions::default();
    let extension = std::any::type_name::<dyn LocatorScreenshotExt>();
    println!("{extension}: {options:?}");
}
