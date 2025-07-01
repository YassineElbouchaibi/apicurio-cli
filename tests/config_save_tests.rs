use apicurio_cli::config;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_save_repo_config_preserves_comments_and_format() {
    let dir = TempDir::new().unwrap();
    let config_path = dir.path().join("apicurioconfig.yaml");

    let initial = r#"# top comment
registries:
  - name: prod
    url: https://example.com

# dependencies section
dependencies:
  - name: svc1
    groupId: com.example
    artifactId: svc1
    version: ^1.0.0
    registry: prod
    outputPath: protos/svc1.proto
"#;
    fs::write(&config_path, initial).unwrap();

    let mut repo = config::load_repo_config(&config_path).unwrap();
    repo.dependencies[0].version = "^2.0.0".to_string();

    config::save_repo_config(&repo, &config_path).unwrap();

    let updated = fs::read_to_string(&config_path).unwrap();
    assert!(updated.contains("# top comment"));
    assert!(updated.contains("# dependencies section"));
    assert!(updated.contains("version: ^2.0.0"));
}
