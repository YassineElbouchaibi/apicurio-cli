use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use std::{collections::HashMap, path::PathBuf};

use crate::{
    config::{load_global_config, load_repo_config},
    constants::{APICURIO_CONFIG, APICURIO_LOCK},
    dependency::Dependency,
    lockfile::{LockFile, LockedDependency},
    registry::RegistryClient,
};

pub async fn run() -> Result<()> {
    // 1) load repo + global + merge registries
    let repo_cfg = load_repo_config(&PathBuf::from(APICURIO_CONFIG))?;
    let global_cfg = load_global_config()?;
    let registries = repo_cfg.merge_registries(global_cfg)?;
    let mut clients = HashMap::new();
    for reg in &registries {
        clients.insert(reg.name.clone(), RegistryClient::new(reg)?);
    }

    // 2) for each declared dependency, resolve & hash
    let mut new_locks = Vec::with_capacity(repo_cfg.dependencies.len());
    for dep_cfg in &repo_cfg.dependencies {
        // parse semver requirement
        let dep = Dependency::from_config(dep_cfg)?;
        let client = &clients[&dep.registry];

        // list+filter versions
        let all_versions = client
            .list_versions(&dep.group_id, &dep.artifact_id)
            .await
            .with_context(|| {
                format!("listing versions for {}/{}", dep.group_id, dep.artifact_id)
            })?;

        let selected = all_versions
            .iter()
            .filter(|v| dep.req.matches(v))
            .max()
            .with_context(|| {
                format!(
                    "no version matching '{}' for dependency '{}'",
                    dep_cfg.version, dep_cfg.name
                )
            })?;

        // download bytes just for hashing
        let data = client
            .download(&dep.group_id, &dep.artifact_id, selected)
            .await
            .with_context(|| format!("downloading content for {} v{}", dep_cfg.name, selected))?;

        // compute sha256
        let sha256 = {
            let mut hasher = Sha256::new();
            hasher.update(&data);
            hex::encode(hasher.finalize())
        };

        new_locks.push(LockedDependency {
            name: dep_cfg.name.clone(),
            registry: dep.registry.clone(),
            resolved_version: selected.to_string(),
            download_url: client.get_download_url(&dep.group_id, &dep.artifact_id, selected),
            sha256,
            output_path: dep.output_path.clone(),
        });
    }

    // 3) load existing lock (if any) & compare
    let lock_path = PathBuf::from(APICURIO_LOCK);
    let needs_write = match LockFile::load(&lock_path) {
        Ok(existing) => {
            existing.locked_dependencies.len() != new_locks.len()
                || !existing
                    .locked_dependencies
                    .iter()
                    .zip(new_locks.iter())
                    .all(|(a, b)| {
                        a.name == b.name
                            && a.registry == b.registry
                            && a.resolved_version == b.resolved_version
                            && a.download_url == b.download_url
                            && a.sha256 == b.sha256
                            && a.output_path == b.output_path
                    })
        }
        Err(_) => true,
    };

    if needs_write {
        let lf = LockFile {
            locked_dependencies: new_locks,
        };
        lf.save(&lock_path)
            .with_context(|| format!("writing {}", lock_path.display()))?;
        println!("🔒 Updated {}", lock_path.display());
    } else {
        println!("🔒 Lock file already up-to-date");
    }

    Ok(())
}
