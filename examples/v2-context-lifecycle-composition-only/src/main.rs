use pwright_composition_chromium_context_lifecycle::{
    ChromiumContextLifecycleProfile, NoopChromiumContextRetirement,
};

fn main() {
    println!(
        "Context lifecycle composition profile: {} bytes; default plugin: {} bytes",
        std::mem::size_of::<ChromiumContextLifecycleProfile>(),
        std::mem::size_of::<NoopChromiumContextRetirement>()
    );
}
