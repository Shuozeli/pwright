use pwright_composition_chromium_page_lifecycle::{
    ChromiumPageLifecycleProfile, NoopChromiumPageRetirement,
};

fn main() {
    println!(
        "Page lifecycle composition profile: {} bytes; default plugin: {} bytes",
        std::mem::size_of::<ChromiumPageLifecycleProfile>(),
        std::mem::size_of::<NoopChromiumPageRetirement>()
    );
}
