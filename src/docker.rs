use anyhow::{anyhow, Result};
use std::{fs, process::Command};
use tracing::info;

use crate::config::AppConfig;

pub fn build_and_transfer_image(config: &AppConfig) -> Result<()> {
    let commands = vec![
        format!(
            "docker build --tag {} --platform=linux/amd64 .",
            config.name
        ),
        format!("docker save -o {}-latest.tar {}", config.name, config.name),
        format!(
            "scp -C {}-latest.tar hobby@{}:./{}/",
            config.name, config.server, config.name
        ),
    ];

    for cmd_str in commands {
        let output = Command::new("sh").arg("-c").arg(&cmd_str).output()?;

        if !output.status.success() {
            return Err(anyhow!(
                "Command failed: {}\nOutput: {}\nError: {}",
                cmd_str,
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            ));
        }
    }

    info!("Image built and transferred successfully");
    fs::remove_file(format!("{}-latest.tar", config.name))?;
    Ok(())
}
