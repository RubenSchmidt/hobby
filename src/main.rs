mod commands;
mod config;
mod deploy;
mod docker;
mod env;
mod launch;
mod setup;

use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing_subscriber;

#[derive(Parser)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Setup { server_addr: String },
    Launch,
    Deploy,
}

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let cli = Cli::parse();

    match cli.command {
        Commands::Setup { server_addr } => {
            setup::setup(server_addr)?;
        }
        Commands::Launch => {
            launch::launch()?;
        }
        Commands::Deploy => {
            deploy::deploy()?;
        }
    }
    Ok(())
}
