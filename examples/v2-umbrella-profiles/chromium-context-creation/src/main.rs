use pwright::chromium_context_creation::{
    ChromiumContextCreationProfile, NoopChromiumContextPublication,
};

fn main() {
    println!(
        "{} {}",
        std::mem::size_of::<ChromiumContextCreationProfile>(),
        std::mem::size_of::<NoopChromiumContextPublication>()
    );
}
