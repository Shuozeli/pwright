use pwright::chromium_page_creation::{
    ChromiumPageCreationProfile, NoopChromiumPagePublication,
};

fn main() {
    println!(
        "{} {}",
        std::mem::size_of::<ChromiumPageCreationProfile>(),
        std::mem::size_of::<NoopChromiumPagePublication>()
    );
}
