use harvest::{Database, EventHandler, RpcServer};
use std::sync::Arc;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
pub struct ServerOptions {
    // short and long flags (-n, --namespace) will be deduced from the field's name
    #[structopt(short, long)]
    namespace: String,

    // short and long flags (-p, --path) will be deduced from the field's name
    #[structopt(short = "a", long)]
    addrs: String,

    // short and long flags (-p, --path) will be deduced from the field's name
    #[structopt(short = "p", long)]
    path: String,
}
// cargo run -- --namespace xx --addrs 0.0.0.0:8080 --path /var/log/container

fn main() {
    // let opt = ServerOptions::from_args();
    // println!("{:?}", opt);
    let event_handler = EventHandler::new();

    let server = RpcServer::new(
        "127.0.0.1:8080".parse().unwrap(),
        Arc::new(Database::new(event_handler)),
    );
    server.listen();
}
