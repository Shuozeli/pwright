use pwright::chromium_handle_lifecycle::ChromiumHandleLifecycleProfile;

fn main() {
    println!(
        "{}",
        std::mem::size_of::<ChromiumHandleLifecycleProfile>()
    );
}
