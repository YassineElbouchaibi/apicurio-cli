use anyhow::Result;
use clap::Subcommand;

pub mod add;
pub mod completions;
pub mod doctor;
pub mod init;
pub mod list;
pub mod pull;
pub mod push;
pub mod registry;
pub mod remove;
pub mod status;
pub mod update;
pub mod verify;

#[derive(Subcommand, Debug)]
pub enum Commands {
    #[command(about = concat!(
        "Scaffold a blank config (and empty lock) in a new repo"
    ))]
    Init,
    #[command(
        about = "Fetch exactly what's in the lock; if no lock, resolve specs ⇒ download ⇒ lock"
    )]
    Pull,
    #[command(
        about = "Re-resolve semver ranges in config to latest matches; download ⇒ overwrite lock"
    )]
    Update,
    #[command(about = "Add a new dependency entry to the config (interactive prompts)")]
    Add { name: String },
    #[command(about = "Remove an existing dependency by name")]
    Remove { name: String },
    #[command(
        about = "Print all configured deps (spec'd & locked versions), and registries (no network)"
    )]
    List,
    #[command(about = "Compare lock vs. latest matching version in registry; flag outdated deps")]
    Status,
    #[command(about = "Re-hash downloaded files & confirm against lockfile hashes")]
    Verify,
    #[command(about = "Subcommand: manage global registries file (add/list/remove)")]
    Registry {
        #[command(subcommand)]
        cmd: registry::RegistryCommands,
    },
    #[command(
        about = "Validate config + lock semantics (semver syntax, missing fields, unreachable URLs)"
    )]
    Doctor,
    #[command(about = "Emit shell completion scripts (bash/zsh/fish)")]
    Completions { shell: String },
    #[command(about = "Publish to registries")]
    Push,
}

pub async fn run(cmd: Commands) -> Result<()> {
    match cmd {
        Commands::Pull => pull::run().await,
        Commands::Update => update::run().await,
        Commands::Init => init::run().await,
        Commands::Add { name } => add::run(name).await,
        Commands::Remove { name } => remove::run(name).await,
        Commands::List => list::run().await,
        Commands::Status => status::run().await,
        Commands::Verify => verify::run().await,
        Commands::Registry { cmd } => registry::run(cmd).await,
        Commands::Doctor => doctor::run().await,
        Commands::Completions { shell } => completions::run(shell),
        Commands::Push => push::run().await,
    }
}
