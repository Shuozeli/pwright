use pwright_composition_chromium_locator::{ChromiumLocatorProfile, ChromiumLocatorTrack};

fn main() {
    println!(
        "Locator composition profile: {} bytes; tracks: {:?}, {:?}",
        std::mem::size_of::<ChromiumLocatorProfile>(),
        ChromiumLocatorTrack::V1,
        ChromiumLocatorTrack::V2
    );
}
