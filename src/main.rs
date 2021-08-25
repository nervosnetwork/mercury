use tokio::{sync::mpsc::channel, task};

use std::{panic, process};

#[tokio::main]
async fn main() {
    let (panic_sender, mut panic_receiver) = channel::<()>(1);

    panic::set_hook(Box::new(move |_info: &panic::PanicInfo| {
        panic_sender.try_send(()).expect("panic_receiver is droped");
    }));

    task::spawn_local(async move {
        if let Some(_) = panic_receiver.recv().await {
            process::exit(-1);
        }
    });

    let mercury = core_cli::Cli::init();
    mercury.start().await;
}
