use pwright_composition_chromium_page_events::ChromiumPageEventsProfile;

fn main() {
    println!(
        "page-events composition profile: {} bytes",
        std::mem::size_of::<ChromiumPageEventsProfile>()
    );
}
