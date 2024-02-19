pub mod raft;

use std::time::Duration;
use ::raft::Raft;
use tokio::runtime::Runtime;
use clap::Parser;
use tokio::task::JoinHandle;
use crate::raft::EbRaft;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(long)]
    raft_addr: String,
    #[arg(long)]
    peer_addr: Option<String>,
}

fn runRaft() -> ::futures::channel::oneshot::Receiver<()> {
    let (s, r) = ::futures::channel::oneshot::channel();

    let rt = ::tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .thread_name("my-custom-name")
        .enable_all()
        .build()
        .expect("could not create runtime");
    rt.enter();


    let task = tokio::task::Builder::new()
        .name("raft main loop")
        .spawn_on(async move {
            EbRaft::run().await;
            s.send(());
            println!("finish");
        }, rt.handle()).expect("...");

    ::std::mem::forget(rt);

    println!("??");

    r
}

#[tokio::main]
async fn main() {
    ::tracing_subscriber::fmt::init();
    println!("Hello, world!");
    println!("{:#?}", runRaft().await);
    println!("sended?");
}
