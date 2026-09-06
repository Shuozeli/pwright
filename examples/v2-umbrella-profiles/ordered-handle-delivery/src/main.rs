use pwright::ordered_handle_delivery::HandleCreationCommand;

fn main() {
    println!(
        "{}",
        <pwright::handle_evaluation::EvaluateHandle as HandleCreationCommand>::operation_name()
    );
}
