use anyhow::Result;
use std::{collections::HashMap, fs, path::PathBuf};

use crate::{
    config::{load_global_config, load_repo_config},
    dependency::Dependency,
    lockfile::{LockFile, LockedDependency},
    registry::RegistryClient,
};
use sha2::{Sha256, Digest};

pub async fn run() -> Result<()> {
    // load configs
    let repo_cfg = load_repo_config(&PathBuf::from("apicurioconfig.yaml"))?;
    let global_cfg = load_global_config()?;
    let regs = repo_cfg.merge_registries(global_cfg)?;

    // build clients
    let mut clients = HashMap::new();
    for r in &regs {
        clients.insert(r.name.clone(), RegistryClient::new(r)?);
    }

    let mut locked: Vec<LockedDependency> = Vec::new();
    // re-resolve every semver range, download, re-lock
    for dep_cfg in &repo_cfg.dependencies {
        let dep = Dependency::from_config(dep_cfg)?;
        let client = &clients[&dep.registry];
        let versions = client
            .list_versions(&dep.group_id, &dep.artifact_id)
            .await?;
        let selected = versions
            .iter()
            .filter(|v| dep.req.matches(v))
            .max()
            .ok_or_else(|| anyhow::anyhow!("no matching version for {}", dep.name))?;
        let data = client
            .download(&dep.group_id, &dep.artifact_id, selected)
            .await?;
        fs::create_dir_all(&dep.output_path)?;
        let file = PathBuf::from(&dep.output_path).join(format!("{}.proto", dep.name));
        fs::write(&file, &data)?;
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

    // save new lockfile
    let lock_path = PathBuf::from("apicuriolock.yaml");
    let lf = LockFile { locked_dependencies: locked };
    lf.save(&lock_path)?;

    println!("✅ update complete");
    Ok(())
}
