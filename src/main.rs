mod commands;
mod config;
mod dependency;
mod lockfile;
mod registry;

use anyhow::Result;
use clap::Parser;

#[derive(Parser)]
#[command(
    name = "apicurio",
    version,
    about = "CLI tool for managing Protobuf artifacts"
)]
struct Cli {
    #[command(subcommand)]
    cmd: Option<commands::Commands>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let cmd = cli.cmd.unwrap_or_else(|| {
        eprintln!("No command provided. Use --help to see available commands.");
        std::process::exit(1);
    });
    commands::run(cmd).await
}
