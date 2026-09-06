use pwright_composition_chromium_page_creation::{
    ChromiumPageCreationProfile, NoopChromiumPagePublication,
};

fn main() {
    println!(
        "Page creation composition profile: {} bytes; default plugin: {} bytes",
        std::mem::size_of::<ChromiumPageCreationProfile>(),
        std::mem::size_of::<NoopChromiumPagePublication>()
    );
}
