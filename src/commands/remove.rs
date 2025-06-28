use anyhow::Result;
use std::{fs, path::PathBuf};
use crate::config::load_repo_config;

pub async fn run(name: String) -> Result<()> {
    let repo_path = PathBuf::from("apicurioconfig.yaml");
    let mut repo = load_repo_config(&repo_path)?;
    let before = repo.dependencies.len();
    repo.dependencies.retain(|d| d.name != name);
    if repo.dependencies.len() == before {
        println!("No dependency named '{}'", name);
    } else {
        let serialized = serde_yaml::to_string(&repo)?;
        fs::write(repo_path, serialized)?;
        println!("✅ removed '{}'", name);
    }
    Ok(())
}
