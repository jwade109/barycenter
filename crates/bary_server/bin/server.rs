use bary_server::BaryServer;
use clap::Parser;
use log::info;

/// Run the test client app
#[derive(Parser, Debug, Default, Clone)]
#[command(version, about, long_about = None)]
pub struct Args {
    #[arg(default_value = "5000")]
    server_port: u16,
    #[arg(long, short)]
    savefile: Option<String>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    simple_logger::init_with_level(log::Level::Info).unwrap();

    let args = Args::parse();

    info!("{args:?}");

    let mut app = BaryServer::new(args.server_port)?;

    if let Some(path) = args.savefile {
        app.load_world(path);
    }

    loop {
        app.update();
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}
