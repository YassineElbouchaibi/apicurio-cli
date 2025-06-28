//! Lock file management for reproducible builds
//!
//! This module handles the creation, loading, and validation of lock files that ensure
//! reproducible builds by recording exact versions, download URLs, and content hashes
//! of all dependencies.
//!
//! ## Lock File Format
//!
//! The lock file (`apicuriolock.yaml`) contains:
//! - Exact resolved versions of all dependencies
//! - Download URLs used to fetch artifacts
//! - SHA256 checksums for integrity verification
//! - Metadata about when the lock was generated
//! - Hash of the configuration that generated the lock
//!
//! ## Integrity Verification
//!
//! Lock files include multiple layers of integrity verification:
//! - Configuration hash to detect config changes
//! - File modification timestamps
//! - SHA256 checksums of downloaded content
//! - Lockfile format version for compatibility

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{fs, path::Path};

/// A locked dependency with exact version and integrity information
///
/// Represents a dependency that has been resolved to an exact version
/// with all information needed for reproducible fetching.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LockedDependency {
    /// Local name/alias of the dependency
    pub name: String,
    /// Registry name where this dependency was resolved
    pub registry: String,
    /// Exact resolved version (no semver ranges)
    pub resolved_version: String,
    /// Full URL used to download the artifact
    pub download_url: String,
    /// SHA256 checksum of the downloaded content
    pub sha256: String,
    /// Local path where the artifact is stored
    pub output_path: String,
    /// Group ID of the artifact
    pub group_id: String,
    /// Artifact ID in the registry
    pub artifact_id: String,
    /// Original version specification from config (e.g., "^1.0.0")
    pub version_spec: String,
}

/// Lock file containing all resolved dependencies and metadata
///
/// The lock file ensures reproducible builds by recording exact versions
/// and integrity information for all dependencies.
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct LockFile {
    /// List of all locked dependencies
    pub locked_dependencies: Vec<LockedDependency>,
    /// Version of the lockfile format for compatibility
    pub lockfile_version: u32,
    /// Hash of the configuration that generated this lock
    pub config_hash: String,
    /// Timestamp when this lock was generated (nanoseconds since epoch)
    pub generated_at: String,
    /// Configuration file modification time (nanoseconds since epoch)
    pub config_modified: Option<String>,
}

impl LockFile {
    /// Load a lock file from disk
    ///
    /// # Arguments
    /// * `path` - Path to the lock file
    ///
    /// # Returns
    /// Parsed lock file structure
    ///
    /// # Errors
    /// Returns error if file cannot be read or parsed as valid YAML
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let data = fs::read_to_string(path)?;
        let lf: LockFile = serde_yaml::from_str(&data)?;
        Ok(lf)
    }

    /// Save the lock file to disk
    ///
    /// # Arguments
    /// * `path` - Path where to save the lock file
    ///
    /// # Errors
    /// Returns error if file cannot be written or serialized
    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        let data = serde_yaml::to_string(self)?;
        fs::write(path, data)?;
        Ok(())
    }

    /// Create a new lockfile with current timestamp and version
    ///
    /// # Arguments
    /// * `locked_dependencies` - Vector of resolved dependencies
    /// * `config_hash` - Hash of the configuration that generated this lock
    #[allow(dead_code)]
    pub fn new(locked_dependencies: Vec<LockedDependency>, config_hash: String) -> Self {
        Self::with_config_modified(locked_dependencies, config_hash, None)
    }

    /// Create a new lockfile with config modification time
    ///
    /// # Arguments
    /// * `locked_dependencies` - Vector of resolved dependencies
    /// * `config_hash` - Hash of the configuration
    /// * `config_modified` - Optional config file modification timestamp
    pub fn with_config_modified(
        locked_dependencies: Vec<LockedDependency>,
        config_hash: String,
        config_modified: Option<String>,
    ) -> Self {
        let now = chrono::Utc::now()
            .timestamp_nanos_opt()
            .unwrap_or(0)
            .to_string();

        Self {
            locked_dependencies,
            lockfile_version: 1,
            config_hash,
            generated_at: now,
            config_modified,
        }
    }

    /// Check if this lockfile is compatible with the given config hash
    pub fn is_compatible_with_config(&self, config_hash: &str) -> bool {
        self.config_hash == config_hash
    }

    /// Check if the lockfile is up-to-date based on config file modification time
    pub fn is_newer_than_config(&self, config_path: &Path) -> anyhow::Result<bool> {
        if let Some(config_modified_str) = &self.config_modified {
            if let Ok(config_modified_nanos) = config_modified_str.parse::<i64>() {
                if let Ok(metadata) = fs::metadata(config_path) {
                    if let Ok(actual_modified) = metadata.modified() {
                        let actual_nanos = chrono::DateTime::<chrono::Utc>::from(actual_modified)
                            .timestamp_nanos_opt()
                            .unwrap_or(0);
                        return Ok(config_modified_nanos >= actual_nanos);
                    }
                }
            }
        }
        // If we can't determine modification times, fall back to hash comparison
        Ok(false)
    }

    /// Enhanced lockfile validation that checks multiple criteria
    #[allow(dead_code)]
    pub fn is_up_to_date(
        &self,
        config_path: &Path,
        current_config_hash: &str,
        dependencies: &[LockedDependency],
    ) -> anyhow::Result<bool> {
        // 1. Check config hash compatibility
        if !self.is_compatible_with_config(current_config_hash) {
            return Ok(false);
        }

        // 2. Check if config file was modified after lockfile was generated
        if !self.is_newer_than_config(config_path)? {
            return Ok(false);
        }

        // 3. Check that dependencies match exactly
        if !self.dependencies_match(dependencies) {
            return Ok(false);
        }

        Ok(true)
    }

    /// Compare two sets of locked dependencies, accounting for order independence
    #[allow(dead_code)]
    pub fn dependencies_match(&self, other_deps: &[LockedDependency]) -> bool {
        if self.locked_dependencies.len() != other_deps.len() {
            return false;
        }

        // Create maps for order-independent comparison
        let self_map: std::collections::HashMap<&str, &LockedDependency> = self
            .locked_dependencies
            .iter()
            .map(|d| (d.name.as_str(), d))
            .collect();
        let other_map: std::collections::HashMap<&str, &LockedDependency> =
            other_deps.iter().map(|d| (d.name.as_str(), d)).collect();

        // Check that all dependencies match exactly
        self_map.len() == other_map.len()
            && self_map.iter().all(|(name, dep)| {
                other_map
                    .get(name)
                    .is_some_and(|other_dep| **dep == **other_dep)
            })
    }

    /// Compute a hash of the relevant configuration that affects locking
    /// This focuses only on the dependency specifications, not formatting/comments
    pub fn compute_config_hash(
        config_content: &str,
        dependencies: &[crate::config::DependencyConfig],
    ) -> String {
        let mut hasher = Sha256::new();

        // Only hash the dependency specifications in a deterministic order
        // This avoids regeneration due to formatting/comment changes
        let mut dep_specs: Vec<String> = dependencies
            .iter()
            .map(|d| {
                format!(
                    "{}:{}:{}:{}:{}:{}",
                    d.name, d.group_id, d.artifact_id, d.version, d.registry, d.output_path
                )
            })
            .collect();
        dep_specs.sort();

        for spec in dep_specs {
            hasher.update(spec.as_bytes());
        }

        // Also include a simplified version of other config that affects dependency resolution
        // Parse the config to extract only relevant fields
        if let Ok(config) = serde_yaml::from_str::<crate::config::RepoConfig>(config_content) {
            // Include registry configurations as they affect resolution
            let mut registry_specs: Vec<String> = config
                .registries
                .iter()
                .map(|r| format!("{}:{}", r.name, r.url))
                .collect();
            registry_specs.sort();

            for spec in registry_specs {
                hasher.update(spec.as_bytes());
            }

            // Include external registries file path if present
            if let Some(ext_file) = &config.external_registries_file {
                hasher.update(ext_file.as_bytes());
            }
        }

        hex::encode(hasher.finalize())
    }

    /// Get the modification time of a config file as nanoseconds since epoch
    pub fn get_config_modification_time(config_path: &Path) -> anyhow::Result<String> {
        let metadata = fs::metadata(config_path)?;
        let modified = metadata.modified()?;
        let nanos = chrono::DateTime::<chrono::Utc>::from(modified)
            .timestamp_nanos_opt()
            .unwrap_or(0);
        Ok(nanos.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_config(dependencies: &[(&str, &str, &str, &str, &str, &str)]) -> String {
        let mut deps = String::new();
        for (name, group_id, artifact_id, version, registry, output_path) in dependencies {
            deps.push_str(&format!(
                r#"
  - name: "{name}"
    groupId: "{group_id}"
    artifactId: "{artifact_id}"
    version: "{version}"
    registry: "{registry}"
    outputPath: "{output_path}"
"#
            ));
        }

        format!(
            r#"externalRegistriesFile: null
registries: []
dependencies:{deps}"#
        )
    }

    fn create_test_locked_dependency(
        name: &str,
        registry: &str,
        resolved_version: &str,
        group_id: &str,
        artifact_id: &str,
        version_spec: &str,
    ) -> LockedDependency {
        LockedDependency {
            name: name.to_string(),
            registry: registry.to_string(),
            resolved_version: resolved_version.to_string(),
            download_url: format!(
                "https://example.com/{group_id}/{artifact_id}/{resolved_version}"
            ),
            sha256: "dummy_hash".to_string(),
            output_path: "./protos".to_string(),
            group_id: group_id.to_string(),
            artifact_id: artifact_id.to_string(),
            version_spec: version_spec.to_string(),
        }
    }

    #[test]
    fn test_config_hash_computation() {
        let config1 = create_test_config(&[(
            "dep1",
            "com.example",
            "service1",
            "1.0.0",
            "registry1",
            "./protos",
        )]);

        let config2 = create_test_config(&[(
            "dep1",
            "com.example",
            "service1",
            "1.0.0",
            "registry1",
            "./protos",
        )]);

        let config3 = create_test_config(&[(
            "dep1",
            "com.example",
            "service1",
            "1.1.0",
            "registry1",
            "./protos",
        )]);

        use crate::config::DependencyConfig;
        let deps1 = vec![DependencyConfig {
            name: "dep1".to_string(),
            group_id: "com.example".to_string(),
            artifact_id: "service1".to_string(),
            version: "1.0.0".to_string(),
            registry: "registry1".to_string(),
            output_path: "./protos".to_string(),
        }];

        let deps3 = vec![DependencyConfig {
            name: "dep1".to_string(),
            group_id: "com.example".to_string(),
            artifact_id: "service1".to_string(),
            version: "1.1.0".to_string(),
            registry: "registry1".to_string(),
            output_path: "./protos".to_string(),
        }];

        let hash1 = LockFile::compute_config_hash(&config1, &deps1);
        let hash2 = LockFile::compute_config_hash(&config2, &deps1);
        let hash3 = LockFile::compute_config_hash(&config3, &deps3);

        assert_eq!(hash1, hash2, "Same config should produce same hash");
        assert_ne!(
            hash1, hash3,
            "Different config should produce different hash"
        );
    }

    #[test]
    fn test_dependencies_match_order_independence() {
        let dep1 = create_test_locked_dependency(
            "dep1",
            "reg1",
            "1.0.0",
            "com.example",
            "service1",
            "^1.0",
        );
        let dep2 = create_test_locked_dependency(
            "dep2",
            "reg1",
            "2.0.0",
            "com.example",
            "service2",
            "^2.0",
        );

        let deps_order1 = vec![dep1.clone(), dep2.clone()];
        let deps_order2 = vec![dep2.clone(), dep1.clone()];

        let lockfile = LockFile::new(deps_order1.clone(), "test_hash".to_string());

        assert!(lockfile.dependencies_match(&deps_order1));
        assert!(
            lockfile.dependencies_match(&deps_order2),
            "Order should not matter"
        );
    }

    #[test]
    fn test_dependencies_match_different_content() {
        let dep1 = create_test_locked_dependency(
            "dep1",
            "reg1",
            "1.0.0",
            "com.example",
            "service1",
            "^1.0",
        );
        let dep2 = create_test_locked_dependency(
            "dep2",
            "reg1",
            "2.0.0",
            "com.example",
            "service2",
            "^2.0",
        );
        let dep1_modified = create_test_locked_dependency(
            "dep1",
            "reg1",
            "1.1.0",
            "com.example",
            "service1",
            "^1.0",
        );

        let deps1 = vec![dep1.clone(), dep2.clone()];
        let deps2 = vec![dep1_modified, dep2.clone()];

        let lockfile = LockFile::new(deps1.clone(), "test_hash".to_string());

        assert!(lockfile.dependencies_match(&deps1));
        assert!(
            !lockfile.dependencies_match(&deps2),
            "Different versions should not match"
        );
    }

    #[test]
    fn test_config_compatibility() {
        let dep1 = create_test_locked_dependency(
            "dep1",
            "reg1",
            "1.0.0",
            "com.example",
            "service1",
            "^1.0",
        );

        let lockfile = LockFile::new(vec![dep1], "test_hash".to_string());

        assert!(lockfile.is_compatible_with_config("test_hash"));
        assert!(!lockfile.is_compatible_with_config("different_hash"));
    }

    #[test]
    fn test_lockfile_serialization() {
        let dep1 = create_test_locked_dependency(
            "dep1",
            "reg1",
            "1.0.0",
            "com.example",
            "service1",
            "^1.0",
        );
        let lockfile = LockFile::new(vec![dep1], "test_hash".to_string());

        let serialized = serde_yaml::to_string(&lockfile).unwrap();
        let deserialized: LockFile = serde_yaml::from_str(&serialized).unwrap();

        assert_eq!(lockfile.config_hash, deserialized.config_hash);
        assert_eq!(lockfile.lockfile_version, deserialized.lockfile_version);
        assert_eq!(
            lockfile.locked_dependencies.len(),
            deserialized.locked_dependencies.len()
        );
        assert!(lockfile.dependencies_match(&deserialized.locked_dependencies));
    }

    #[test]
    fn test_empty_dependencies() {
        let lockfile = LockFile::new(vec![], "test_hash".to_string());

        assert!(lockfile.dependencies_match(&[]));
        assert!(
            !lockfile.dependencies_match(&[create_test_locked_dependency(
                "dep1",
                "reg1",
                "1.0.0",
                "com.example",
                "service1",
                "^1.0"
            )])
        );
    }

    #[test]
    fn test_missing_dependency() {
        let dep1 = create_test_locked_dependency(
            "dep1",
            "reg1",
            "1.0.0",
            "com.example",
            "service1",
            "^1.0",
        );
        let dep2 = create_test_locked_dependency(
            "dep2",
            "reg1",
            "2.0.0",
            "com.example",
            "service2",
            "^2.0",
        );

        let lockfile = LockFile::new(vec![dep1.clone(), dep2.clone()], "test_hash".to_string());

        assert!(!lockfile.dependencies_match(&[dep1])); // Missing dep2
        assert!(!lockfile.dependencies_match(&[dep2])); // Missing dep1
    }

    #[test]
    fn test_config_hash_deterministic_ordering() {
        // Test that dependency order in config doesn't affect hash
        let deps1 = vec![
            crate::config::DependencyConfig {
                name: "dep_a".to_string(),
                group_id: "com.example".to_string(),
                artifact_id: "service_a".to_string(),
                version: "1.0.0".to_string(),
                registry: "registry1".to_string(),
                output_path: "./protos".to_string(),
            },
            crate::config::DependencyConfig {
                name: "dep_b".to_string(),
                group_id: "com.example".to_string(),
                artifact_id: "service_b".to_string(),
                version: "2.0.0".to_string(),
                registry: "registry1".to_string(),
                output_path: "./protos".to_string(),
            },
        ];

        let deps2 = vec![deps1[1].clone(), deps1[0].clone()]; // Reverse order

        let config_content = "test config";
        let hash1 = LockFile::compute_config_hash(config_content, &deps1);
        let hash2 = LockFile::compute_config_hash(config_content, &deps2);

        assert_eq!(hash1, hash2, "Config hash should be order-independent");
    }

    #[test]
    fn test_enhanced_config_hash_ignores_formatting() {
        // Test that the improved hash function ignores formatting differences
        let deps = vec![crate::config::DependencyConfig {
            name: "dep1".to_string(),
            group_id: "com.example".to_string(),
            artifact_id: "service1".to_string(),
            version: "1.0.0".to_string(),
            registry: "registry1".to_string(),
            output_path: "./protos".to_string(),
        }];

        // These configs have different formatting but same semantic content
        let config1 = r#"
externalRegistriesFile: null
registries: []
dependencies:
  - name: dep1
    groupId: com.example
    artifactId: service1
    version: "1.0.0"
    registry: registry1
    outputPath: ./protos
"#;

        let config2 = r#"
externalRegistriesFile: null
registries: []
# This is a comment
dependencies:
  - name: dep1
    groupId: com.example
    artifactId: service1
    version: "1.0.0"
    registry: registry1
    outputPath: ./protos
# Another comment
"#;

        let hash1 = LockFile::compute_config_hash(config1, &deps);
        let hash2 = LockFile::compute_config_hash(config2, &deps);

        assert_eq!(
            hash1, hash2,
            "Config hash should ignore comments and formatting"
        );
    }

    #[test]
    fn test_with_config_modified() {
        let dep1 = create_test_locked_dependency(
            "dep1",
            "reg1",
            "1.0.0",
            "com.example",
            "service1",
            "^1.0",
        );
        let config_modified = Some("1234567890123456789".to_string());

        let lockfile = LockFile::with_config_modified(
            vec![dep1],
            "test_hash".to_string(),
            config_modified.clone(),
        );

        assert_eq!(lockfile.config_modified, config_modified);
        assert!(lockfile.generated_at.parse::<i64>().is_ok());
    }

    #[test]
    fn test_is_newer_than_config_with_missing_data() {
        let dep1 = create_test_locked_dependency(
            "dep1",
            "reg1",
            "1.0.0",
            "com.example",
            "service1",
            "^1.0",
        );

        // Test lockfile without config_modified
        let lockfile = LockFile::new(vec![dep1.clone()], "test_hash".to_string());
        let result = lockfile
            .is_newer_than_config(Path::new("nonexistent"))
            .unwrap();
        assert!(
            !result,
            "Should return false when config_modified is missing"
        );

        // Test lockfile with invalid config_modified
        let mut lockfile_invalid = LockFile::new(vec![dep1], "test_hash".to_string());
        lockfile_invalid.config_modified = Some("invalid_number".to_string());
        let result = lockfile_invalid
            .is_newer_than_config(Path::new("nonexistent"))
            .unwrap();
        assert!(
            !result,
            "Should return false when config_modified is invalid"
        );
    }

    #[test]
    fn test_lockfile_backwards_compatibility() {
        // Test that old lockfiles without config_modified still work
        let old_lockfile_yaml = r#"
lockfileVersion: 1
configHash: "test_hash"
generatedAt: "1234567890"
lockedDependencies:
  - name: "dep1"
    registry: "reg1"
    resolvedVersion: "1.0.0"
    downloadUrl: "https://example.com/dep1"
    sha256: "dummy_hash"
    outputPath: "./protos"
    groupId: "com.example"
    artifactId: "service1"
    versionSpec: "^1.0"
"#;

        let lockfile: LockFile = serde_yaml::from_str(old_lockfile_yaml).unwrap();
        assert!(lockfile.config_modified.is_none());
        assert_eq!(lockfile.config_hash, "test_hash");
        assert_eq!(lockfile.locked_dependencies.len(), 1);
    }

    #[test]
    fn test_robust_dependency_matching() {
        let dep1_v1 = create_test_locked_dependency(
            "dep1",
            "reg1",
            "1.0.0",
            "com.example",
            "service1",
            "^1.0",
        );
        let dep1_v2 = create_test_locked_dependency(
            "dep1",
            "reg1",
            "1.0.1",
            "com.example",
            "service1",
            "^1.0",
        );
        let dep2 = create_test_locked_dependency(
            "dep2",
            "reg1",
            "2.0.0",
            "com.example",
            "service2",
            "^2.0",
        );

        let lockfile = LockFile::new(vec![dep1_v1.clone(), dep2.clone()], "test_hash".to_string());

        // Exact match should work
        assert!(lockfile.dependencies_match(&[dep1_v1.clone(), dep2.clone()]));
        assert!(lockfile.dependencies_match(&[dep2.clone(), dep1_v1.clone()])); // Order independence

        // Different version should fail
        assert!(!lockfile.dependencies_match(&[dep1_v2, dep2.clone()]));

        // Missing dependency should fail
        assert!(!lockfile.dependencies_match(&[dep1_v1.clone()]));

        // Extra dependency should fail
        let dep3 = create_test_locked_dependency(
            "dep3",
            "reg1",
            "3.0.0",
            "com.example",
            "service3",
            "^3.0",
        );
        assert!(!lockfile.dependencies_match(&[dep1_v1.clone(), dep2.clone(), dep3]));
    }
}
