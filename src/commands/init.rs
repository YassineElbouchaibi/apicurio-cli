use anyhow::Result;
use std::{fs, path::Path};

pub async fn run() -> Result<()> {
    let cfg = Path::new("apicurioconfig.yaml");
    if cfg.exists() {
        println!("Config already exists at {}", cfg.display());
    } else {
        let template = r#"externalRegistriesFile: ${APICURIO_REGISTRIES_PATH:-}
registries: []
dependencies: []"#;
        fs::write(cfg, template)?;
        println!("Created {}", cfg.display());
    }

    let lock = Path::new("apicuriolock.yaml");
    if !lock.exists() {
        fs::write(lock, "lockedDependencies: []")?;
        println!("Created {}", lock.display());
    }

    Ok(())
}
