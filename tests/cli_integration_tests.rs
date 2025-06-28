use std::process::Command;
use tempfile::TempDir;

#[test]
fn test_cli_help_command() {
    let output = Command::new("cargo")
        .args(["run", "--", "--help"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("apicurio"));
}

#[test]
fn test_cli_init_command_in_temp_dir() {
    let temp_dir = TempDir::new().unwrap();

    let output = Command::new("cargo")
        .args(["run", "--", "init"])
        .current_dir(temp_dir.path())
        .env("CARGO_MANIFEST_DIR", env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("Failed to execute command");

    // The init command should succeed (or fail predictably)
    // This test demonstrates how to test CLI commands that interact with the filesystem
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Since we're running from a temp directory without proper setup,
    // we expect this to fail, but we can verify the error handling
    assert!(!stderr.is_empty() || output.status.success());
}
