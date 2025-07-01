use anyhow::Result;
use std::{fs, path::Path};

use crate::config::{save_repo_config, RepoConfig};
use crate::constants::{APICURIO_CONFIG, APICURIO_LOCK};

pub async fn run() -> Result<()> {
    let cfg = Path::new(APICURIO_CONFIG);
    if cfg.exists() {
        println!("Config already exists at {}", cfg.display());
    } else {
        let repo = RepoConfig {
            external_registries_file: Some("${APICURIO_REGISTRIES_PATH:-}".into()),
            ..Default::default()
        };
        save_repo_config(&repo, cfg)?;
        println!("Created {}", cfg.display());
    }

    let lock = Path::new(APICURIO_LOCK);
    if !lock.exists() {
        fs::write(lock, "lockedDependencies: []")?;
        println!("Created {}", lock.display());
    }

    Ok(())
}
