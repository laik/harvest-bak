use common::Result;
use harvest::Harvest;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
pub struct ServerOptions {
    // short and long flags (-n, --namespace) will be deduced from the field's name
    #[structopt(short, long)]
    namespace: String,

    // // short and long flags (-s, --server) will be deduced from the field's name
    #[structopt(short = "o", long)]
    output: String,

    // short and long flags (-d, --dir) will be deduced from the field's name
    #[structopt(short = "d", long)]
    dir: String,
}
// cargo run -- --namespace xx --path /var/log/container --server http://localhost:9999/

fn main() -> Result<()> {
    // let opt = ServerOptions::from_args();
    // println!("{:?}", opt);

    Harvest::new(
        "default".to_owned(),
        "/Users/dxp/workspace/go/src/github.com/laik/harvest/tmp".to_owned(),
        "http://localhost:9999/".to_owned(),
        "node1".to_owned(),
    )
    .start()
}
