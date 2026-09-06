use pwright_plugin_ordered_handle_delivery::HandleCreationCommand;

fn main() {
    println!(
        "{}",
        <pwright_cap_handle_evaluation::EvaluateHandle as HandleCreationCommand>::operation_name()
    );
}
