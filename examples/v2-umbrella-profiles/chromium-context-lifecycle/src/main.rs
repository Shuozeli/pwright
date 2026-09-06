use pwright::chromium_context_lifecycle::{
    ChromiumContextLifecycleProfile, NoopChromiumContextRetirement,
};

fn main() {
    println!(
        "{} {}",
        std::mem::size_of::<ChromiumContextLifecycleProfile>(),
        std::mem::size_of::<NoopChromiumContextRetirement>()
    );
}
