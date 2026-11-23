use std::path::PathBuf;

use bary_server::BaryServer;
use clap::Parser;
use log::info;

/// Run the test client app
#[derive(Parser, Debug, Default, Clone)]
#[command(version, about, long_about = None)]
pub struct Args {
    /// Path containing save files and scenarios to load
    save_files_dir: PathBuf,
    /// Port on which to host the server
    #[arg(default_value = "5000")]
    server_port: u16,
    /// Savefile to load immediately
    #[arg(long, short)]
    savefile: Option<String>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    simple_logger::init_with_level(log::Level::Info).unwrap();

    let args = Args::parse();

    info!("{args:?}");

    let mut server = BaryServer::new(args.save_files_dir, args.server_port)?;

    if let Some(path) = args.savefile {
        server.load_world(path);
    }

    loop {
        server.update();
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}
