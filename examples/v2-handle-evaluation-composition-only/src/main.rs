use pwright_composition_chromium_handle_evaluation::ChromiumHandleEvaluationProfile;

fn main() {
    println!(
        "Handle evaluation composition profile: {} bytes",
        std::mem::size_of::<ChromiumHandleEvaluationProfile>()
    );
}
