use common::Result;
use db::Database;
use event::EventHandler;
use harvest::Harvest;
use std::sync::Arc;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
pub struct ServerOptions {
    // short and long flags (-n, --namespace) will be deduced from the field's name
    #[structopt(short, long)]
    namespace: String,

    // short and long flags (-d, --dir) will be deduced from the field's name
    #[structopt(short = "d", long)]
    dir: String,
}
// cargo run -- --namespace xx --path /var/log/container

fn main() -> Result<()> {
    // let opt = ServerOptions::from_args();
    // println!("{:?}", opt);

    Harvest::new("default".to_owned(), "/var/log/pods".to_owned()).start()
}
