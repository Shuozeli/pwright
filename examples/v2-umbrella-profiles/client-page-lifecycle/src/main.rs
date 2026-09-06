use pwright::client_page_lifecycle::{PageCloseExt, PageRunBeforeUnloadExt};

fn main() {
    let _ = (
        std::any::type_name::<dyn PageCloseExt>(),
        std::any::type_name::<dyn PageRunBeforeUnloadExt>(),
    );
}
