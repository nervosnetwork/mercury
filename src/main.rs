#[tokio::main(flavor = "multi_thread", worker_threads = 16)]
async fn main() {
    std::panic::set_hook(Box::new(move |info| {
        log::error!("panic occurred {:?}", info);
        std::process::exit(-1);
    }));

    let mercury = core_cli::Cli::init(clap::crate_version!().to_string());
    mercury.start().await;
}
