#[tokio::main(flavor = "multi_thread")]
async fn main() {
    std::panic::set_hook(Box::new(move |info| {
        println!("panic occurred {:?}", info);
        log::error!("panic occurred {:?}", info);
        std::process::exit(-1);
    }));

    let mercury = core_cli::Cli::init();
    mercury.start().await;
}
