use pwright::client_content::PageScreenshotExt;
use pwright::content::ScreenshotOptions;

fn main() {
    let options = ScreenshotOptions::default();
    let extension = std::any::type_name::<dyn PageScreenshotExt>();
    println!("{extension}: {options:?}");
}
