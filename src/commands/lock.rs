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
    let config_path = PathBuf::from(APICURIO_CONFIG);
    let config_content = std::fs::read_to_string(&config_path)
        .with_context(|| format!("reading config from {}", config_path.display()))?;
    let repo_cfg = load_repo_config(&config_path)?;
    let global_cfg = load_global_config()?;
    let registries = repo_cfg.merge_registries(global_cfg)?;

    // Compute config hash for lock integrity
    let config_hash = LockFile::compute_config_hash(&config_content, &repo_cfg.dependencies);

    let mut clients = HashMap::new();
    for reg in &registries {
        clients.insert(reg.name.clone(), RegistryClient::new(reg)?);
    }

    // 2) Check if existing lock is up-to-date with enhanced validation
    let lock_path = PathBuf::from(APICURIO_LOCK);
    if let Ok(existing_lock) = LockFile::load(&lock_path) {
        // First, quick check: is config hash the same?
        if existing_lock.is_compatible_with_config(&config_hash) {
            // Second, check modification time if available
            if existing_lock
                .is_newer_than_config(&config_path)
                .unwrap_or(false)
            {
                // Third, verify all dependencies can still be resolved
                if verify_lock_is_still_valid(&existing_lock, &clients).await? {
                    println!("🔒 Lock file already up-to-date");
                    return Ok(());
                } else {
                    println!("🔓 Lock file outdated: some dependencies are no longer available");
                }
            } else {
                println!("🔓 Lock file outdated: config file has been modified");
            }
        } else {
            println!("🔓 Lock file outdated: config hash changed");
        }
    }

    // 3) for each declared dependency, resolve & hash
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
            group_id: dep.group_id.clone(),
            artifact_id: dep.artifact_id.clone(),
            version_spec: dep_cfg.version.clone(),
        });
    }

    // 4) Create new lockfile with metadata including config modification time
    let config_modified = LockFile::get_config_modification_time(&config_path).ok();
    let lf = LockFile::with_config_modified(new_locks, config_hash, config_modified);
    lf.save(&lock_path)
        .with_context(|| format!("writing {}", lock_path.display()))?;
    println!("🔒 Updated {}", lock_path.display());

    Ok(())
}

/// Verify that an existing lock file can still be resolved with the same versions
/// This performs a more lightweight check than re-resolving all dependencies
async fn verify_lock_is_still_valid(
    lock: &LockFile,
    clients: &HashMap<String, RegistryClient>,
) -> Result<bool> {
    // Quick optimization: if the lockfile is very recent (< 5 minutes),
    // trust it without checking registries
    if let Ok(generated_nanos) = lock.generated_at.parse::<i64>() {
        let now_nanos = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);

        // If lockfile was generated within the last 5 minutes, trust it
        let five_minutes_nanos = 5 * 60 * 1_000_000_000i64; // 5 minutes in nanoseconds
        if now_nanos.saturating_sub(generated_nanos) < five_minutes_nanos {
            return Ok(true);
        }
    }

    // Otherwise, verify each dependency can still be resolved
    for locked_dep in &lock.locked_dependencies {
        let client = match clients.get(&locked_dep.registry) {
            Some(c) => c,
            None => {
                eprintln!(
                    "Warning: Registry '{}' is no longer configured",
                    locked_dep.registry
                );
                return Ok(false);
            }
        };

        // Check if the exact version is still available
        match client
            .list_versions(&locked_dep.group_id, &locked_dep.artifact_id)
            .await
        {
            Ok(versions) => {
                if !versions
                    .iter()
                    .any(|v| v.to_string() == locked_dep.resolved_version)
                {
                    eprintln!(
                        "Warning: Version '{}' of '{}:{}' is no longer available",
                        locked_dep.resolved_version, locked_dep.group_id, locked_dep.artifact_id
                    );
                    return Ok(false);
                }
            }
            Err(e) => {
                eprintln!(
                    "Warning: Failed to check availability of '{}:{}': {}",
                    locked_dep.group_id, locked_dep.artifact_id, e
                );
                // On network errors, etc., we'll be conservative and re-generate
                return Ok(false);
            }
        }
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tokio;

    #[test]
    fn test_verify_lock_is_still_valid_with_missing_registry() {
        // Create lockfile with old timestamp to bypass recent optimization
        let mut lock = LockFile::new(vec![], "test_hash".to_string());
        lock.generated_at = "1000000000000000000".to_string(); // Very old timestamp
        lock.locked_dependencies.push(LockedDependency {
            name: "test_dep".to_string(),
            registry: "missing_registry".to_string(),
            resolved_version: "1.0.0".to_string(),
            download_url: "https://example.com/test".to_string(),
            sha256: "test_hash".to_string(),
            output_path: "./protos".to_string(),
            group_id: "com.example".to_string(),
            artifact_id: "test".to_string(),
            version_spec: "^1.0".to_string(),
        });

        let clients = HashMap::new(); // Empty clients map

        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(verify_lock_is_still_valid(&lock, &clients));

        assert!(result.is_ok());
        assert!(
            !result.unwrap(),
            "Should return false when registry is missing"
        );
    }

    // Note: Full integration tests would require mocking the RegistryClient
    // which is beyond the scope of this improvement but could be added later
}
