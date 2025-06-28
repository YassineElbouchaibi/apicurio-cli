use anyhow::Result;
use std::{collections::HashMap, fs, path::PathBuf};

use crate::{
    config::{load_global_config, load_repo_config},
    constants::{APICURIO_CONFIG, APICURIO_LOCK},
    dependency::Dependency,
    lockfile::{LockFile, LockedDependency},
    registry::RegistryClient,
};
use sha2::{Digest, Sha256};

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

    let lock_path = PathBuf::from(APICURIO_LOCK);
    let mut locked: Vec<LockedDependency> = Vec::new();

    if lock_path.exists() {
        // re-download exactly what's in the lockfile
        let lf = LockFile::load(&lock_path)?;
        for d in lf.locked_dependencies {
            let client = &clients[&d.registry];
            // download by exact URL, but we know API path from download_url
            let data = client
                .client
                .get(&d.download_url)
                .send()
                .await?
                .bytes()
                .await?;
            fs::create_dir_all(&d.output_path)?;
            let file = PathBuf::from(&d.output_path).join(format!("{}.proto", d.name));
            fs::write(&file, &data)?;
            locked.push(d);
        }
    } else {
        // first‐time: resolve semver, download, lock
        for dep_cfg in &repo_cfg.dependencies {
            let dep = Dependency::from_config(dep_cfg)?;
            let client = &clients[&dep.registry];
            let versions = client
                .list_versions(&dep.group_id, &dep.artifact_id)
                .await?;
            // pick highest matching
            let selected = versions
                .iter()
                .filter(|v| dep.req.matches(v))
                .max()
                .ok_or_else(|| anyhow::anyhow!("no matching version for {}", dep.name))?;
            let data = client
                .download(&dep.group_id, &dep.artifact_id, selected)
                .await?;
            // write file
            fs::create_dir_all(&dep.output_path)?;
            let file = PathBuf::from(&dep.output_path).join(format!("{}.proto", dep.name));
            fs::write(&file, &data)?;
            // hash it
            let sha = {
                let mut h = Sha256::new();
                h.update(&data);
                hex::encode(h.finalize())
            };
            let url = format!(
                "{}/apis/registry/v2/groups/{}/artifacts/{}/versions/{}/content",
                client.base_url, dep.group_id, dep.artifact_id, selected
            );
            locked.push(LockedDependency {
                name: dep.name.clone(),
                registry: dep.registry.clone(),
                resolved_version: selected.to_string(),
                download_url: url,
                sha256: sha,
                output_path: dep.output_path.clone(),
            });
        }
        // write lockfile
        let lf = LockFile {
            locked_dependencies: locked.clone(),
        };
        lf.save(&lock_path)?;
    }

    println!("✅ pull complete");
    Ok(())
}
