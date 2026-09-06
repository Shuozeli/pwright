use pwright::chromium_page_lifecycle::{
    ChromiumPageLifecycleProfile, NoopChromiumPageRetirement,
};

fn main() {
    println!(
        "{} {}",
        std::mem::size_of::<ChromiumPageLifecycleProfile>(),
        std::mem::size_of::<NoopChromiumPageRetirement>()
    );
}
