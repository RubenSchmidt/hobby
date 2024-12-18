use crate::config::AppConfig;
use anyhow::{anyhow, Result};
use homedir::my_home;
use std::{fs, process::Command};
use tracing::info;

pub fn get_env_file_hash(config: &AppConfig) -> Result<String> {
    if let Some(env) = &config.env {
        if env.file.is_empty() {
            return Ok("".to_string());
        }

        let env_path = format!("./{}", env.file);
        let env_content = fs::read(&env_path)?;
        Ok(format!("{:x}", md5::compute(&env_content)))
    } else {
        Ok("".to_string())
    }
}

pub fn encrypt_and_upload_env_file(config: &mut AppConfig) -> Result<()> {
    if let Some(env) = &config.env {
        if env.file.is_empty() {
            return Ok(());
        }

        let hash = get_env_file_hash(config)?;

        if hash == env.hash {
            info!("Environment file has not changed, skipping encryption and upload");
            return Ok(());
        }

        info!("Encrypting environment file...");
        let config_dir = my_home()?
            .ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?
            .join(".config/hobby");
        let public_key_path = config_dir.join("key.pub");
        let public_key = fs::read_to_string(public_key_path)?.trim().to_string();

        let env_path = format!("./{}", env.file);

        let encrypt_command = format!(
            "sops encrypt --age {} {} > encrypted.env",
            public_key, env_path
        );
        let output = Command::new("sh")
            .arg("-c")
            .arg(&encrypt_command)
            .output()?;

        if !output.status.success() {
            return Err(anyhow!(
                "Failed to encrypt env file: {}\nOutput: {}\nError: {}",
                encrypt_command,
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        info!("Syncing encrypted environment file to server...");
        let sync_command = format!(
            "rsync -v encrypted.env hobby@{}:./{}",
            config.server, config.name
        );
        let output = Command::new("sh").arg("-c").arg(&sync_command).output()?;

        if !output.status.success() {
            return Err(anyhow!(
                "Failed to sync env file: {}\nOutput: {}\nError: {}",
                sync_command,
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        fs::remove_file("encrypted.env")?;
        if let Some(env) = &mut config.env {
            env.hash = hash;
        }
    }
    Ok(())
}
