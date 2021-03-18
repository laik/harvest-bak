use common::Result;
use harvest::Harvest;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
pub struct ServerOptions {
    // short and long flags (-n, --namespace) will be deduced from the field's name
    #[structopt(short, long)]
    namespace: String,

    // // short and long flags (-s, --api-server) will be deduced from the field's name
    #[structopt(short = "s", long)]
    api_server: String,

    // short and long flags (-d, --dir) will be deduced from the field's name
    #[structopt(short = "d", long)]
    dir: String,

    // short and long flags (-h, --node) will be deduced from the field's name
    #[structopt(short = "h", long)]
    host: String,
}
// cargo run -- --namespace xx --dir /var/log/container --api-server http://localhost:9999/ --host node1

fn main() -> Result<()> {
    let opt = ServerOptions::from_args();
    println!("recv args {:?}", opt);

    Harvest::new(&opt.namespace, &opt.dir, &opt.api_server, &opt.host).start(1000)
}
