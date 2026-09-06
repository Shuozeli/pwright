use pwright::client_handle_evaluation::HandleProperty;

fn main() {
    let _ = std::any::type_name::<HandleProperty<(), ()>>();
}
