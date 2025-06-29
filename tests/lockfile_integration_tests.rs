use apicurio_cli::{config, lockfile};
use std::fs;
use tempfile::TempDir;

fn create_test_config_file(dir: &TempDir, content: &str) -> std::path::PathBuf {
    let config_path = dir.path().join("apicurioconfig.yaml");
    fs::write(&config_path, content).unwrap();
    config_path
}

fn create_test_lockfile(dir: &TempDir, lockfile: &lockfile::LockFile) -> std::path::PathBuf {
    let lock_path = dir.path().join("apicuriolock.yaml");
    lockfile.save(&lock_path).unwrap();
    lock_path
}

#[test]
fn test_lockfile_regeneration_scenarios() {
    let temp_dir = TempDir::new().unwrap();

    // Create initial config
    let config_content = r#"
externalRegistriesFile: null
registries: []
dependencies:
  - name: "service1"
    groupId: "com.example"
    artifactId: "service1"
    version: "^1.0.0"
    registry: "default"
    outputPath: "./protos/service1.proto"
"#;

    let config_path = create_test_config_file(&temp_dir, config_content);

    let deps = vec![config::DependencyConfig {
        name: "service1".to_string(),
        group_id: Some("com.example".to_string()),
        artifact_id: Some("service1".to_string()),
        version: "^1.0.0".to_string(),
        registry: "default".to_string(),
        output_path: "./protos/service1.proto".to_string(),
        resolve_references: None,
    }];

    // Create lockfile
    let config_hash = lockfile::LockFile::compute_config_hash(config_content, &deps);
    let config_modified = lockfile::LockFile::get_config_modification_time(&config_path).ok();

    let locked_dep = lockfile::LockedDependency {
        name: "service1".to_string(),
        registry: "default".to_string(),
        resolved_version: "1.0.5".to_string(),
        download_url: "https://example.com/service1/1.0.5".to_string(),
        sha256: "abcd1234".to_string(),
        output_path: "./protos/service1.proto".to_string(),
        group_id: "com.example".to_string(),
        artifact_id: "service1".to_string(),
        version_spec: "^1.0.0".to_string(),
        is_transitive: false,
    };

    let lockfile = lockfile::LockFile::with_config_modified(
        vec![locked_dep.clone()],
        config_hash.clone(),
        config_modified,
    );

    let _lock_path = create_test_lockfile(&temp_dir, &lockfile);

    // Test 1: Same config should be compatible
    assert!(lockfile.is_compatible_with_config(&config_hash));

    // Test 2: Config modification time check
    assert!(lockfile.is_newer_than_config(&config_path).unwrap());

    // Test 3: Dependencies match
    assert!(lockfile.dependencies_match(&[locked_dep.clone()]));

    // Test 4: Modified config should trigger regeneration
    let modified_config = r#"
externalRegistriesFile: null
registries: []
dependencies:
  - name: "service1"
    groupId: "com.example"
    artifactId: "service1"
    version: "^1.1.0"  # Version changed
    registry: "default"
    outputPath: "./protos/service1.proto"
"#;

    let modified_deps = vec![config::DependencyConfig {
        name: "service1".to_string(),
        group_id: Some("com.example".to_string()),
        artifact_id: Some("service1".to_string()),
        version: "^1.1.0".to_string(), // Changed version
        registry: "default".to_string(),
        output_path: "./protos/service1.proto".to_string(),
        resolve_references: None,
    }];

    let new_config_hash = lockfile::LockFile::compute_config_hash(modified_config, &modified_deps);
    assert_ne!(
        config_hash, new_config_hash,
        "Config hash should change when version requirements change"
    );
    assert!(!lockfile.is_compatible_with_config(&new_config_hash));
}

#[test]
fn test_formatting_changes_dont_trigger_regeneration() {
    let _temp_dir = TempDir::new().unwrap();

    let deps = vec![config::DependencyConfig {
        name: "service1".to_string(),
        group_id: Some("com.example".to_string()),
        artifact_id: Some("service1".to_string()),
        version: "^1.0.0".to_string(),
        registry: "default".to_string(),
        output_path: "./protos/service1.proto".to_string(),
        resolve_references: None,
    }];

    // Original config
    let config1 = r#"
externalRegistriesFile: null
registries: []
dependencies:
  - name: "service1"
    groupId: "com.example"
    artifactId: "service1"
    version: "^1.0.0"
    registry: "default"
    outputPath: "./protos/service1.proto"
"#;

    // Same config with different formatting and comments
    let config2 = r#"
# Added a comment
externalRegistriesFile: null
registries: []  # Empty registries
dependencies:
  - name: "service1"
    groupId: "com.example"
    artifactId: "service1"
    version: "^1.0.0"
    registry: "default"
    outputPath: "./protos"
    # End of dependency
"#;

    let hash1 = lockfile::LockFile::compute_config_hash(config1, &deps);
    let hash2 = lockfile::LockFile::compute_config_hash(config2, &deps);

    assert_eq!(
        hash1, hash2,
        "Hash should be the same for formatting-only changes"
    );
}

#[test]
fn test_registry_changes_trigger_regeneration() {
    let deps = vec![config::DependencyConfig {
        name: "service1".to_string(),
        group_id: Some("com.example".to_string()),
        artifact_id: Some("service1".to_string()),
        version: "^1.0.0".to_string(),
        registry: "default".to_string(),
        output_path: "./protos".to_string(),
        resolve_references: None,
    }];

    // Config with one registry
    let config1 = r#"
externalRegistriesFile: null
registries:
  - name: "default"
    url: "https://registry1.example.com"
dependencies:
  - name: "service1"
    groupId: "com.example"
    artifactId: "service1"
    version: "^1.0.0"
    registry: "default"
    outputPath: "./protos"
"#;

    // Config with different registry URL
    let config2 = r#"
externalRegistriesFile: null
registries:
  - name: "default"
    url: "https://registry2.example.com"  # Different URL
dependencies:
  - name: "service1"
    groupId: "com.example"
    artifactId: "service1"
    version: "^1.0.0"
    registry: "default"
    outputPath: "./protos"
"#;

    let hash1 = lockfile::LockFile::compute_config_hash(config1, &deps);
    let hash2 = lockfile::LockFile::compute_config_hash(config2, &deps);

    assert_ne!(hash1, hash2, "Hash should change when registry URLs change");
}

#[test]
fn test_external_registry_file_changes_trigger_regeneration() {
    let deps = vec![config::DependencyConfig {
        name: "service1".to_string(),
        group_id: Some("com.example".to_string()),
        artifact_id: Some("service1".to_string()),
        version: "^1.0.0".to_string(),
        registry: "default".to_string(),
        output_path: "./protos".to_string(),
        resolve_references: None,
    }];

    // Config without external registries file
    let config1 = r#"
externalRegistriesFile: null
registries: []
dependencies:
  - name: "service1"
    groupId: "com.example"
    artifactId: "service1"
    version: "^1.0.0"
    registry: "default"
    outputPath: "./protos"
"#;

    // Config with external registries file
    let config2 = r#"
externalRegistriesFile: "external-registries.yaml"
registries: []
dependencies:
  - name: "service1"
    groupId: "com.example"
    artifactId: "service1"
    version: "^1.0.0"
    registry: "default"
    outputPath: "./protos"
"#;

    let hash1 = lockfile::LockFile::compute_config_hash(config1, &deps);
    let hash2 = lockfile::LockFile::compute_config_hash(config2, &deps);

    assert_ne!(
        hash1, hash2,
        "Hash should change when external registries file is added"
    );
}
