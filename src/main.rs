use tokio_compat::FutureExt;

#[tokio::main]
async fn main() {
    let mercury = core_cli::Cli::init();
    mercury.start().compat().await;
}
