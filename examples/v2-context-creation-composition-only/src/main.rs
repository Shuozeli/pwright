use pwright_composition_chromium_context_creation::{
    ChromiumContextCreationProfile, NoopChromiumContextPublication,
};

fn main() {
    println!(
        "Context creation composition profile: {} bytes; default plugin: {} bytes",
        std::mem::size_of::<ChromiumContextCreationProfile>(),
        std::mem::size_of::<NoopChromiumContextPublication>()
    );
}
