use crate::{constants::APICURIO_LOCK, lockfile::LockFile};
use anyhow::{Result, anyhow};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};

pub async fn run() -> Result<()> {
    let lock = LockFile::load(&PathBuf::from(APICURIO_LOCK))?;
    let mut all_ok = true;

    for ld in &lock.locked_dependencies {
        let file = PathBuf::from(&ld.output_path).join(format!("{}.proto", ld.name));
        if !file.exists() {
            println!("❌ missing file for {}: {}", ld.name, file.display());
            all_ok = false;
            continue;
        }
        let data = fs::read(&file)?;
        let mut hasher = Sha256::new();
        hasher.update(&data);
        let sha = hex::encode(hasher.finalize());
        if sha != ld.sha256 {
            println!(
                "❌ hash mismatch {}: expected={}, got={}",
                ld.name, ld.sha256, sha
            );
            all_ok = false;
        } else {
            println!("✔️  {} OK", ld.name);
        }
    }

    if !all_ok {
        return Err(anyhow!("verification failed"));
    }
    Ok(())
}
