#[tokio::main]
async fn main() {
    std::panic::set_hook(Box::new(move |_| {
        std::process::exit(-1);
    }));

    let mercury = core_cli::Cli::init();
    mercury.start().await;
}
