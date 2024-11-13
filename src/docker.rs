use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs, process::Command};
use tracing::info;

use crate::config::AppConfig;

#[derive(Serialize, Deserialize)]
pub struct DockerService {
    pub image: String,
    pub restart: String,
    pub labels: HashMap<String, String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub environment: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volumes: Option<Vec<String>>,
    pub networks: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct DockerNetwork {
    pub external: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DockerVolume {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct DockerComposeFile {
    pub services: HashMap<String, DockerService>,
    pub networks: HashMap<String, DockerNetwork>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volumes: Option<HashMap<String, ()>>,
}

pub fn build_and_transfer_image(config: &AppConfig) -> Result<()> {
    let commands = vec![
        format!(
            "docker build --tag {} --platform=linux/arm64 .",
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

pub fn create_docker_env(file: &str) -> Result<HashMap<String, String>> {
    // Implement environment variable creation logic here
    let env_data = fs::read_to_string(file)?;
    let env_vars: Vec<String> = env_data
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| line.split('=').next().unwrap().to_string())
        .collect();

    let docker_env: Vec<String> = env_vars
        .iter()
        .map(|var| format!("{}=${{{}}}", var, var))
        .collect();

    let mut env_map = HashMap::new();
    for env_str in docker_env.iter() {
        let parts: Vec<&str> = env_str.split('=').collect();
        if parts.len() == 2 {
            env_map.insert(parts[0].to_string(), parts[1].to_string());
        }
    }

    Ok(env_map)
}

pub fn build_compose_config(config: &AppConfig) -> Result<DockerComposeFile> {
    info!("Building docker-compose file...");
    let mut service = DockerService {
        image: config.name.clone(),
        restart: "unless-stopped".to_string(),
        labels: {
            let mut labels = HashMap::new();
            labels.insert("caddy".to_string(), config.url.clone());
            labels.insert(
                "caddy.reverse_proxy".to_string(),
                format!("{{{{upstreams {}}}}}", config.port),
            );
            labels
        },
        networks: vec!["caddy".to_string()],
        environment: None,
        volumes: config.volumes.clone(),
    };

    if let Some(env_config) = &config.env {
        if !env_config.file.is_empty() {
            let env_map = create_docker_env(&env_config.file)?;
            service.environment = Some(env_map);
        }
    }

    let compose = DockerComposeFile {
        services: {
            let mut services = HashMap::new();
            services.insert(config.name.clone(), service);
            services
        },
        networks: {
            let mut networks = HashMap::new();
            networks.insert("caddy".to_string(), DockerNetwork { external: true });
            networks
        },
        volumes: {
            let mut volumes: HashMap<String, ()> = HashMap::new();
            if let Some(vols) = &config.volumes {
                for v in vols {
                    let parts: Vec<&str> = v.split(":").collect();
                    if parts.len() == 2 {
                        volumes.insert(parts[0].to_string(), ());
                    }
                }
            }
            Some(volumes)
        },
    };

    Ok(compose)
}

pub fn write_docker_compose_file(config: &DockerComposeFile) -> Result<()> {
    let data = serde_yaml::to_string(config)?;
    fs::write("docker-compose.yaml", data)?;
    Ok(())
}

pub fn transfer_compose_file(config: &AppConfig) -> Result<()> {
    info!("Transferring docker-compose file...");

    let output = Command::new("sh")
        .arg("-c")
        .arg(format!(
            "scp -C docker-compose.yaml hobby@{}:./{}/",
            config.server,
            config.name // Replace with actual server address
        ))
        .output()?;

    if !output.status.success() {
        return Err(anyhow!(
            "Failed to transfer docker-compose file:\nOutput: {}\nError: {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    info!("docker-compose file transferred successfully");
    Ok(())
}
