use pwright::client_context_creation::BrowserNewContextExt;

fn main() {
    println!("{}", std::any::type_name::<dyn BrowserNewContextExt<Context = ()>>());
}
