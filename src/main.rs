#[tokio::main]
async fn main() {
    let mercury = core_cli::Cli::init();
    mercury.start().await;
}
