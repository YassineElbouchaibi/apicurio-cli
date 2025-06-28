use crate::Cli;
use anyhow::Result;
use clap::CommandFactory;
use clap_complete::{
    generate,
    shells::{Bash, Fish, Zsh},
};
use std::io;

pub fn run(shell: String) -> Result<()> {
    let mut cmd = Cli::command();
    match shell.as_str() {
        "bash" => generate(Bash, &mut cmd, "apicurio", &mut io::stdout()),
        "zsh" => generate(Zsh, &mut cmd, "apicurio", &mut io::stdout()),
        "fish" => generate(Fish, &mut cmd, "apicurio", &mut io::stdout()),
        other => {
            eprintln!("unsupported shell '{other}', choose: bash, zsh, fish");
            std::process::exit(1);
        }
    }
    Ok(())
}
