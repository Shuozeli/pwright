use pwright::client_evaluation::FrameEvaluationExt;
use pwright::evaluation::EvaluateParams;

fn main() {
    let request = EvaluateParams::expression("document.title");
    let extension_is_installed = std::any::type_name::<dyn FrameEvaluationExt>();
    println!("{extension_is_installed}: {}", request.source());
}
