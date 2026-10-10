use bary_core::prelude::*;
use clap::Parser;
use log::info;

use bary_server::{Application, HeadlessServerApp};

/// Run the test client app
#[derive(Parser, Debug, Default, Clone)]
#[command(version, about, long_about = None)]
pub struct Args {
    savefile: String,
    #[arg(default_value = "5000")]
    server_port: u16,
}

fn main() -> BaryResult<()> {
    let args = Args::parse();

    simple_logger::init_with_level(log::Level::Info).unwrap();

    info!("Starting dedicated server...");

    HeadlessServerApp::new(args.savefile, args.server_port, 10)?.spin_forever();

    Ok(())
}
