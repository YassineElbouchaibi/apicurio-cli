use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
};

use crate::{
    config::{load_global_config, load_repo_config},
    constants::{APICURIO_CONFIG, APICURIO_LOCK},
    dependency::Dependency,
    lockfile::{resolve_output_path, LockFile, LockedDependency},
    registry::RegistryClient,
};

/// Represents a dependency to be resolved (either direct or transitive)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct DependencyToResolve {
    group_id: String,
    artifact_id: String,
    version_req: String, // For direct deps, this is semver. For transitive, exact version
    registry: String,
    output_path: Option<String>, // None for transitive deps
    is_transitive: bool,
    depth: u32,
}

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
    let existing_lock = if let Ok(existing_lock) = LockFile::load(&lock_path) {
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
        Some(existing_lock)
    } else {
        None
    };

    // 3) Build initial set of dependencies to resolve
    let mut dependencies_to_resolve = Vec::new();

    // Add direct dependencies from config
    for dep_cfg in &repo_cfg.dependencies {
        let dep = Dependency::from_config(dep_cfg)?;
        dependencies_to_resolve.push(DependencyToResolve {
            group_id: dep.group_id.clone(),
            artifact_id: dep.artifact_id.clone(),
            version_req: dep_cfg.version.clone(),
            registry: dep.registry.clone(),
            output_path: Some(dep.output_path.clone()),
            is_transitive: false,
            depth: 0,
        });
    }

    // 4) Resolve all dependencies including transitive references
    let mut resolved_dependencies = HashMap::new();
    let mut processed = HashSet::new();

    while let Some(dep_to_resolve) = dependencies_to_resolve.pop() {
        let key = format!(
            "{}:{}:{}",
            dep_to_resolve.registry, dep_to_resolve.group_id, dep_to_resolve.artifact_id
        );

        // Skip if already processed
        if processed.contains(&key) {
            continue;
        }
        processed.insert(key.clone());

        // Skip if depth exceeds maximum
        if dep_to_resolve.depth > repo_cfg.reference_resolution.max_depth {
            eprintln!(
                "Warning: Skipping reference resolution for {} at depth {} (exceeds max depth {})",
                key, dep_to_resolve.depth, repo_cfg.reference_resolution.max_depth
            );
            continue;
        }

        let client = &clients[&dep_to_resolve.registry];

        // Resolve version
        let resolved_version = if dep_to_resolve.is_transitive {
            // For transitive deps, version_req is already exact
            semver::Version::parse(&dep_to_resolve.version_req)?
        } else {
            // For direct deps, resolve semver range
            let dep = Dependency {
                name: format!("{}/{}", dep_to_resolve.group_id, dep_to_resolve.artifact_id),
                group_id: dep_to_resolve.group_id.clone(),
                artifact_id: dep_to_resolve.artifact_id.clone(),
                req: semver::VersionReq::parse(&dep_to_resolve.version_req)?,
                registry: dep_to_resolve.registry.clone(),
                output_path: dep_to_resolve.output_path.clone().unwrap_or_default(),
            };

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
                        dep_to_resolve.version_req, dep.name
                    )
                })?;
            selected.clone()
        };

        // Download content for hashing
        let data = client
            .download(
                &dep_to_resolve.group_id,
                &dep_to_resolve.artifact_id,
                &resolved_version,
            )
            .await
            .with_context(|| {
                format!(
                    "downloading content for {}:{} v{}",
                    dep_to_resolve.group_id, dep_to_resolve.artifact_id, resolved_version
                )
            })?;

        // Compute SHA256
        let sha256 = {
            let mut hasher = Sha256::new();
            hasher.update(&data);
            hex::encode(hasher.finalize())
        };

        // Determine output path
        let output_path = if let Some(path) = dep_to_resolve.output_path {
            Some(path)
        } else {
            // Generate path for transitive dependency using pattern and overrides
            let metadata = client
                .get_artifact_metadata(&dep_to_resolve.group_id, &dep_to_resolve.artifact_id)
                .await?;
            resolve_output_path(
                &repo_cfg.reference_resolution.output_pattern,
                &repo_cfg.reference_resolution.output_overrides,
                &dep_to_resolve.registry,
                &dep_to_resolve.group_id,
                &dep_to_resolve.artifact_id,
                &resolved_version.to_string(),
                &metadata.artifact_type,
            )
        };

        // Skip this dependency if it's mapped to null (excluded from resolution)
        let output_path = match output_path {
            Some(path) => path,
            None => {
                println!(
                    "  ⏭️  Skipping transitive dependency {}:{} (mapped to null)",
                    dep_to_resolve.group_id, dep_to_resolve.artifact_id
                );
                continue; // Skip to next dependency
            }
        };

        // Create locked dependency
        let locked_dep = LockedDependency {
            name: if dep_to_resolve.is_transitive {
                format!("{}/{}", dep_to_resolve.group_id, dep_to_resolve.artifact_id)
            } else {
                // Find the original name from config
                repo_cfg
                    .dependencies
                    .iter()
                    .find(|cfg| {
                        let dep = Dependency::from_config(cfg).unwrap();
                        dep.group_id == dep_to_resolve.group_id
                            && dep.artifact_id == dep_to_resolve.artifact_id
                    })
                    .map(|cfg| cfg.name.clone())
                    .unwrap_or_else(|| {
                        format!("{}/{}", dep_to_resolve.group_id, dep_to_resolve.artifact_id)
                    })
            },
            registry: dep_to_resolve.registry.clone(),
            resolved_version: resolved_version.to_string(),
            download_url: client.get_download_url(
                &dep_to_resolve.group_id,
                &dep_to_resolve.artifact_id,
                &resolved_version,
            ),
            sha256,
            output_path,
            group_id: dep_to_resolve.group_id.clone(),
            artifact_id: dep_to_resolve.artifact_id.clone(),
            version_spec: dep_to_resolve.version_req.clone(),
            is_transitive: dep_to_resolve.is_transitive,
        };

        resolved_dependencies.insert(key, locked_dep);

        // Determine if reference resolution should be enabled for this dependency
        let should_resolve_references = if dep_to_resolve.is_transitive {
            // For transitive dependencies, always use global setting
            repo_cfg.reference_resolution.enabled
        } else {
            // For direct dependencies, check per-dependency override first
            let original_dep_config = repo_cfg.dependencies.iter().find(|cfg| {
                let dep = Dependency::from_config(cfg).unwrap();
                dep.group_id == dep_to_resolve.group_id
                    && dep.artifact_id == dep_to_resolve.artifact_id
            });

            match original_dep_config.and_then(|cfg| cfg.resolve_references) {
                Some(override_setting) => override_setting,
                None => repo_cfg.reference_resolution.enabled,
            }
        };

        // If reference resolution is enabled, get version references
        if should_resolve_references
            && dep_to_resolve.depth < repo_cfg.reference_resolution.max_depth
        {
            match client
                .get_version_references(
                    &dep_to_resolve.group_id,
                    &dep_to_resolve.artifact_id,
                    &resolved_version,
                    None,
                )
                .await
            {
                Ok(references) => {
                    for reference in references {
                        // Use "default" as the group_id if the reference doesn't specify one
                        let ref_group_id = reference.group_id.as_deref().unwrap_or("default");

                        let ref_key = format!(
                            "{}:{}:{}",
                            dep_to_resolve.registry, ref_group_id, reference.artifact_id
                        );

                        // Only add if not already processed or in queue
                        if !processed.contains(&ref_key)
                            && !dependencies_to_resolve.iter().any(|d| {
                                format!("{}:{}:{}", d.registry, d.group_id, d.artifact_id)
                                    == ref_key
                            })
                        {
                            dependencies_to_resolve.push(DependencyToResolve {
                                group_id: ref_group_id.to_string(),
                                artifact_id: reference.artifact_id,
                                version_req: reference.version, // References use exact versions
                                registry: dep_to_resolve.registry.clone(), // Use same registry as parent
                                output_path: None, // Will be generated using pattern
                                is_transitive: true,
                                depth: dep_to_resolve.depth + 1,
                            });
                        }
                    }
                }
                Err(e) => {
                    eprintln!(
                        "Warning: Failed to get version references for {}:{} v{}: {}",
                        dep_to_resolve.group_id, dep_to_resolve.artifact_id, resolved_version, e
                    );
                }
            }
        }
    }

    // Convert resolved dependencies to vector
    let mut new_locks: Vec<LockedDependency> = resolved_dependencies.into_values().collect();

    // Sort to ensure consistent ordering (direct deps first, then alphabetical)
    new_locks.sort_by(|a, b| match (a.is_transitive, b.is_transitive) {
        (false, true) => std::cmp::Ordering::Less,
        (true, false) => std::cmp::Ordering::Greater,
        _ => a.name.cmp(&b.name),
    });

    // 4) Create new lockfile with metadata including config modification time
    let config_modified = LockFile::get_config_modification_time(&config_path).ok();
    let lf = LockFile::with_config_modified(new_locks, config_hash, config_modified);

    // 5) Clean up old output paths if they changed
    if let Some(ref old_lock) = existing_lock {
        cleanup_changed_output_paths(&old_lock.locked_dependencies, &lf.locked_dependencies)?;
    }

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

/// Clean up old output files when their paths change during locking
fn cleanup_changed_output_paths(
    old_dependencies: &[LockedDependency],
    new_dependencies: &[LockedDependency],
) -> Result<()> {
    use std::collections::HashMap;

    // Create a map of dependency name to output path for old and new dependencies
    let old_paths: HashMap<&str, &str> = old_dependencies
        .iter()
        .map(|dep| (dep.name.as_str(), dep.output_path.as_str()))
        .collect();

    let new_paths: HashMap<&str, &str> = new_dependencies
        .iter()
        .map(|dep| (dep.name.as_str(), dep.output_path.as_str()))
        .collect();

    // Check for dependencies with changed output paths
    for (dep_name, old_path) in &old_paths {
        if let Some(new_path) = new_paths.get(dep_name) {
            // If the dependency still exists but the output path changed
            if old_path != new_path {
                let old_file = PathBuf::from(old_path);
                if old_file.exists() {
                    match std::fs::remove_file(&old_file) {
                        Ok(()) => {
                            println!("🗑️  Removed old output file: {old_path}");
                        }
                        Err(e) => {
                            eprintln!(
                                "Warning: Failed to remove old output file '{old_path}': {e}"
                            );
                        }
                    }

                    // Also try to remove empty parent directories
                    if let Some(parent) = old_file.parent() {
                        let _ = remove_empty_parent_dirs(parent);
                    }
                }
            }
        } else {
            // Dependency was removed entirely - clean up its output file
            let old_file = PathBuf::from(old_path);
            if old_file.exists() {
                match std::fs::remove_file(&old_file) {
                    Ok(()) => {
                        println!(
                            "🗑️  Removed output file for removed dependency '{dep_name}': {old_path}"
                        );
                    }
                    Err(e) => {
                        eprintln!(
                            "Warning: Failed to remove output file for removed dependency '{dep_name}': {e}"
                        );
                    }
                }

                // Also try to remove empty parent directories
                if let Some(parent) = old_file.parent() {
                    let _ = remove_empty_parent_dirs(parent);
                }
            }
        }
    }

    Ok(())
}

/// Recursively remove empty parent directories up to the current working directory
fn remove_empty_parent_dirs(dir: &std::path::Path) -> Result<()> {
    // Don't try to remove the current working directory or root
    let cwd = std::env::current_dir().unwrap_or_default();
    if dir == cwd || dir.parent().is_none() {
        return Ok(());
    }

    // Only remove if the directory is empty
    if let Ok(mut entries) = std::fs::read_dir(dir) {
        if entries.next().is_none() {
            // Directory is empty, try to remove it
            match std::fs::remove_dir(dir) {
                Ok(()) => {
                    println!("🗑️  Removed empty directory: {}", dir.display());
                    // Recursively try to remove parent directories
                    if let Some(parent) = dir.parent() {
                        let _ = remove_empty_parent_dirs(parent);
                    }
                }
                Err(_) => {
                    // Ignore errors when removing directories (might not have permissions, etc.)
                }
            }
        }
    }

    Ok(())
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
            is_transitive: false,
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

    #[test]
    fn test_cleanup_changed_output_paths() {
        use std::fs;
        use tempfile::TempDir;

        // Create a temporary directory for testing
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Create old and new file paths
        let old_path = temp_path.join("old").join("types.proto");
        let new_path = temp_path.join("new").join("types.proto");

        // Create the old file and its parent directory
        fs::create_dir_all(old_path.parent().unwrap()).unwrap();
        fs::write(&old_path, "old content").unwrap();

        // Create old and new dependencies with different output paths
        let old_deps = vec![LockedDependency {
            name: "test_dep".to_string(),
            registry: "local".to_string(),
            resolved_version: "1.0.0".to_string(),
            download_url: "http://localhost/test".to_string(),
            sha256: "test_hash".to_string(),
            output_path: old_path.to_string_lossy().to_string(),
            group_id: "com.example".to_string(),
            artifact_id: "test".to_string(),
            version_spec: "^1.0".to_string(),
            is_transitive: false,
        }];

        let new_deps = vec![LockedDependency {
            name: "test_dep".to_string(),
            registry: "local".to_string(),
            resolved_version: "1.0.0".to_string(),
            download_url: "http://localhost/test".to_string(),
            sha256: "test_hash".to_string(),
            output_path: new_path.to_string_lossy().to_string(),
            group_id: "com.example".to_string(),
            artifact_id: "test".to_string(),
            version_spec: "^1.0".to_string(),
            is_transitive: false,
        }];

        // Verify old file exists before cleanup
        assert!(old_path.exists());

        // Run cleanup
        cleanup_changed_output_paths(&old_deps, &new_deps).unwrap();

        // Verify old file was removed
        assert!(!old_path.exists());

        // Verify old directory was also removed since it's empty
        assert!(!old_path.parent().unwrap().exists());
    }

    #[test]
    fn test_cleanup_removed_dependency() {
        use std::fs;
        use tempfile::TempDir;

        // Create a temporary directory for testing
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Create old file path
        let old_path = temp_path.join("removed").join("types.proto");

        // Create the old file and its parent directory
        fs::create_dir_all(old_path.parent().unwrap()).unwrap();
        fs::write(&old_path, "old content").unwrap();

        // Create old dependency that will be removed
        let old_deps = vec![LockedDependency {
            name: "removed_dep".to_string(),
            registry: "local".to_string(),
            resolved_version: "1.0.0".to_string(),
            download_url: "http://localhost/test".to_string(),
            sha256: "test_hash".to_string(),
            output_path: old_path.to_string_lossy().to_string(),
            group_id: "com.example".to_string(),
            artifact_id: "test".to_string(),
            version_spec: "^1.0".to_string(),
            is_transitive: false,
        }];

        let new_deps = vec![]; // Empty - dependency removed

        // Verify old file exists before cleanup
        assert!(old_path.exists());

        // Run cleanup
        cleanup_changed_output_paths(&old_deps, &new_deps).unwrap();

        // Verify old file was removed
        assert!(!old_path.exists());

        // Verify old directory was also removed since it's empty
        assert!(!old_path.parent().unwrap().exists());
    }

    #[test]
    fn test_cleanup_unchanged_output_paths() {
        use std::fs;
        use tempfile::TempDir;

        // Create a temporary directory for testing
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Create file path that won't change
        let file_path = temp_path.join("unchanged").join("types.proto");

        // Create the file and its parent directory
        fs::create_dir_all(file_path.parent().unwrap()).unwrap();
        fs::write(&file_path, "content").unwrap();

        // Create dependencies with same output path
        let deps = vec![LockedDependency {
            name: "unchanged_dep".to_string(),
            registry: "local".to_string(),
            resolved_version: "1.0.0".to_string(),
            download_url: "http://localhost/test".to_string(),
            sha256: "test_hash".to_string(),
            output_path: file_path.to_string_lossy().to_string(),
            group_id: "com.example".to_string(),
            artifact_id: "test".to_string(),
            version_spec: "^1.0".to_string(),
            is_transitive: false,
        }];

        // Verify file exists before cleanup
        assert!(file_path.exists());

        // Run cleanup with same old and new deps
        cleanup_changed_output_paths(&deps, &deps).unwrap();

        // Verify file still exists (unchanged)
        assert!(file_path.exists());
    }
}
