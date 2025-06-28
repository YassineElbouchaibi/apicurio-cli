use crate::{
    config::DependencyConfig,
    config::{load_global_config, load_repo_config},
};
use anyhow::{Result, anyhow};
use std::{
    fs,
    io::{Write, stdin, stdout},
    path::PathBuf,
};

fn prompt(msg: &str) -> Result<String> {
    print!("{}: ", msg);
    stdout().flush()?;
    let mut input = String::new();
    stdin().read_line(&mut input)?;
    let val = input.trim().to_string();
    if val.is_empty() {
        Err(anyhow!("{} cannot be empty", msg))
    } else {
        Ok(val)
    }
}

pub async fn run(name: String) -> Result<()> {
    // load and merge registries to show choices
    let repo_path = PathBuf::from("apicurioconfig.yaml");
    let mut repo = load_repo_config(&repo_path)?;
    let global = load_global_config()?;
    let regs = repo.merge_registries(global)?;
    let names: Vec<_> = regs.iter().map(|r| r.name.clone()).collect();
    println!("Available registries: {}", names.join(", "));

    // prompt for fields
    let group_id = prompt("Group ID")?;
    let artifact_id = prompt("Artifact ID")?;
    let version = prompt("Version (semver)")?;
    let registry = prompt("Registry (one of above)")?;
    if !names.contains(&registry) {
        return Err(anyhow!("Invalid registry: {}", registry));
    }
    let output_path = prompt("Output path")?;

    // append and save
    repo.dependencies.push(DependencyConfig {
        name,
        group_id: group_id,
        artifact_id: artifact_id,
        version,
        registry,
        output_path: output_path,
    });
    let serialized = serde_yaml::to_string(&repo)?;
    fs::write(repo_path, serialized)?;
    println!("✅ added dependency");
    Ok(())
}
