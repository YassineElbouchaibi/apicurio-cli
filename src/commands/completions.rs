use clap::CommandFactory;
use clap_complete::{generate, shells::{Bash, Zsh, Fish}};
use std::io;
use anyhow::Result;
use crate::Cli;

pub fn run(shell: String) -> Result<()> {
    let mut cmd = Cli::command();
    match shell.as_str() {
        "bash" => generate(Bash, &mut cmd, "apicurio", &mut io::stdout()),
        "zsh"  => generate(Zsh, &mut cmd, "apicurio", &mut io::stdout()),
        "fish" => generate(Fish, &mut cmd, "apicurio", &mut io::stdout()),
        other => {
            eprintln!("unsupported shell '{}', choose: bash, zsh, fish", other);
            std::process::exit(1);
        }
    }
    Ok(())
}
