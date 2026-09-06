use pwright_composition_chromium_handle_lifecycle::ChromiumHandleLifecycleProfile;

fn main() {
    println!(
        "Handle lifecycle composition profile: {} bytes",
        std::mem::size_of::<ChromiumHandleLifecycleProfile>()
    );
}
