use anyhow::{anyhow, Result};
use homedir::my_home;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs};

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

#[derive(Serialize, Deserialize, Default)]
pub struct AppConfig {
    pub name: String,
    pub server: String,
    pub url: String,
    pub port: u16,
    pub env: Option<EnvConfig>,
    pub volumes: Option<Vec<String>>,
    pub version: String,
}

#[derive(Serialize, Deserialize, Default)]
pub struct EnvConfig {
    pub file: String,
    pub hash: String,
}

pub fn load_app_config() -> Result<AppConfig> {
    // Implement application configuration loading logic here
    let config_data = fs::read_to_string("./hobby.yml")?;
    let config: AppConfig = serde_yaml::from_str(&config_data)?;
    Ok(config)
}

pub fn get_config_dir() -> Result<std::path::PathBuf> {
    let config_dir = my_home()?
        .ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?
        .join(".config/hobby");

    Ok(config_dir)
}

pub fn load_secret_key() -> Result<String> {
    let config_dir = get_config_dir()?;
    let secret_key_path = config_dir.join("key.txt");

    // Read the secret key from the file
    let secret_key = fs::read_to_string(&secret_key_path)
        .map_err(|e| anyhow::anyhow!("Failed to read secret key file: {}", e))?;

    // Return the secret key as a SecretString
    Ok(secret_key)
}

pub fn validate_environment() -> Result<()> {
    // Implement environment validation logic here
    if !fs::metadata("./Dockerfile").is_ok() {
        return Err(anyhow!("No Dockerfile found in current directory"));
    }

    if !fs::metadata("./hobby.yml").is_ok() {
        return Err(anyhow!(
            "hobby config is missing - run 'init' command first"
        ));
    }

    Ok(())
}
