use anyhow::Result;
use std::{collections::HashMap, fs, path::PathBuf};

use crate::{
    config::{load_global_config, load_repo_config},
    constants::{APICURIO_CONFIG, APICURIO_LOCK},
    lockfile::LockFile,
    registry::RegistryClient,
};

pub async fn run() -> Result<()> {
    // 1) load configs
    let repo_cfg = load_repo_config(&PathBuf::from(APICURIO_CONFIG))?;
    let global_cfg = load_global_config()?;
    let regs = repo_cfg.merge_registries(global_cfg)?;
    // build clients
    let mut clients = HashMap::new();
    for r in &regs {
        clients.insert(r.name.clone(), RegistryClient::new(r)?);
    }

    crate::commands::lock::run().await?;
    let lock_path = PathBuf::from(APICURIO_LOCK);
    let lock_file = LockFile::load(&lock_path)?;
    for dependency in lock_file.locked_dependencies {
        let client = &clients[&dependency.registry];
        // download by exact URL, but we know API path from download_url
        let data = client
            .client
            .get(&dependency.download_url)
            .send()
            .await?
            .bytes()
            .await?;
        let file_path = PathBuf::from(&dependency.output_path);
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&file_path, &data)?;
    }

    println!("✅ pull complete");
    Ok(())
}
