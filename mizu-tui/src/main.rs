use cursive::views::TextView;
use gumdrop::Options;

#[derive(Options)]
struct Opt {
    // the first free argument is treated as Tezos address
    #[options(free)]
    arguments: Vec<String>,
    #[options(help = "path to application data")]
    appdata: Option<String>,
    #[options(help = "path to local Tezos mock")]
    mock: Option<String>,
    #[options(help = "Tezos RPC server")]
    rpc: Option<String>,
}

fn main() {
    let mut siv = cursive::default();
    siv.add_global_callback('q', |s| s.quit());
    siv.add_layer(TextView::new("press q to quit."));
    siv.run();
}
