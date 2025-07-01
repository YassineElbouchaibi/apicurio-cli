use anyhow::Result;
use std::{collections::HashMap, fs, path::PathBuf};

use crate::{
    config::{load_global_config, load_repo_config},
    constants::{APICURIO_CONFIG, APICURIO_LOCK},
    dependency::Dependency,
    lockfile::{LockFile, LockedDependency},
    output_path,
    registry::RegistryClient,
};
use sha2::{Digest, Sha256};

pub async fn run() -> Result<()> {
    // load configs
    let repo_cfg = load_repo_config(&PathBuf::from(APICURIO_CONFIG))?;
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
        let dep = Dependency::from_config_with_defaults(dep_cfg, &repo_cfg.dependency_defaults)?;
        let client = &clients[&dep.registry];
        let versions = client
            .list_versions(&dep.group_id, &dep.artifact_id)
            .await?;
        let selected = versions
            .iter()
            .filter(|v| dep.req.matches(v))
            .max()
            .ok_or_else(|| anyhow::anyhow!("no matching version for {}", dep.name))?;
        let metadata = client
            .get_artifact_metadata(&dep.group_id, &dep.artifact_id)
            .await?;
        let output_path = dep.output_path.clone().unwrap_or_else(|| {
            let pattern = repo_cfg
                .dependency_defaults
                .output_patterns
                .resolve(&metadata.artifact_type, None);
            output_path::generate_output_path(
                &pattern,
                &dep.group_id,
                &dep.artifact_id,
                &selected.to_string(),
                &metadata.artifact_type,
            )
        });

        let data = client
            .download(&dep.group_id, &dep.artifact_id, selected)
            .await?;
        let file_path = PathBuf::from(&output_path);
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&file_path, &data)?;
        let sha = {
            let mut h = Sha256::new();
            h.update(&data);
            hex::encode(h.finalize())
        };
        locked.push(LockedDependency {
            name: dep.name.clone(),
            registry: dep.registry.clone(),
            resolved_version: selected.to_string(),
            download_url: client.get_download_url(&dep.group_id, &dep.artifact_id, selected),
            sha256: sha,
            output_path,
            group_id: dep.group_id.clone(),
            artifact_id: dep.artifact_id.clone(),
            version_spec: dep_cfg.version.clone(),
            is_transitive: false,
        });
    }

    // save new lockfile with config modification time
    let lock_path = PathBuf::from(APICURIO_LOCK);
    let config_path = PathBuf::from(APICURIO_CONFIG);
    let config_content = std::fs::read_to_string(&config_path)?;
    let config_hash = LockFile::compute_config_hash(&config_content, &repo_cfg.dependencies);
    let config_modified = LockFile::get_config_modification_time(&config_path).ok();
    let lf = LockFile::with_config_modified(locked, config_hash, config_modified);
    lf.save(&lock_path)?;

    println!("✅ update complete");
    Ok(())
}
