use crate::commands::{connect_ssh, run_ssh_commands};
use crate::config::{
    load_app_config, load_secret_key, validate_environment, AppConfig, DockerComposeFile,
    DockerNetwork, DockerService,
};
use crate::docker::build_and_transfer_image;
use crate::env;
use anyhow::{anyhow, Result};
use serde_yaml;
use std::collections::HashMap;
use std::fs;
use std::process::Command;
use tracing::info;

pub fn launch() -> Result<()> {
    let start = std::time::Instant::now();

    validate_environment()?;

    info!("Loading app config...");
    let mut app_config = load_app_config()?;

    let secret_key = load_secret_key()?;

    let mut service = DockerService {
        image: app_config.name.clone(),
        restart: "unless-stopped".to_string(),
        labels: {
            let mut labels = HashMap::new();
            labels.insert("caddy".to_string(), app_config.url.clone());
            labels.insert(
                "caddy.reverse_proxy".to_string(),
                format!("{{{{upstreams {}}}}}", app_config.port),
            );
            labels
        },
        networks: vec!["caddy".to_string()],
        environment: None,
        volumes: app_config.volumes.clone(),
    };

    if let Some(env_config) = &app_config.env {
        if !env_config.file.is_empty() {
            let env_vec = create_docker_env(&env_config.file)?;
            let mut env_map = HashMap::new();
            for env_str in env_vec {
                let parts: Vec<&str> = env_str.split('=').collect();
                if parts.len() == 2 {
                    env_map.insert(parts[0].to_string(), parts[1].to_string());
                }
            }
            service.environment = Some(env_map);
        }
    }

    let compose = DockerComposeFile {
        services: {
            let mut services = HashMap::new();
            services.insert(app_config.name.clone(), service);
            services
        },
        networks: {
            let mut networks = HashMap::new();
            networks.insert("caddy".to_string(), DockerNetwork { external: true });
            networks
        },
        volumes: {
            let mut volumes: HashMap<String, ()> = HashMap::new();
            if let Some(vols) = &app_config.volumes {
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

    write_docker_compose_file(&compose)?;

    deploy_application(&mut app_config, &secret_key)?;

    save_application_config(&app_config)?;

    fs::remove_file("docker-compose.yaml")?;

    info!(
        "Application launched successfully in {:?}",
        start.elapsed().as_secs()
    );
    info!("Application available at: {}", app_config.url);
    Ok(())
}

fn create_docker_env(file: &str) -> Result<Vec<String>> {
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

    Ok(docker_env)
}

fn write_docker_compose_file(config: &DockerComposeFile) -> Result<()> {
    let data = serde_yaml::to_string(config)?;
    fs::write("docker-compose.yaml", data)?;
    Ok(())
}

fn deploy_application(config: &mut AppConfig, secret_key: &String) -> Result<()> {
    info!("Deploying application...");

    let session = connect_ssh("hobby", &config.server)?;

    run_ssh_commands(&session, &[&format!("mkdir -p {}", config.name)])?;

    // Encrypt and upload environment file
    if let Some(env_config) = &config.env {
        if !env_config.file.is_empty() {
            env::encrypt_and_upload_env_file(config)?;
        }
    }

    info!("Building and transferring Docker image...");
    build_and_transfer_image(config)?;

    info!("Transferring docker-compose file...");
    transfer_compose_file(config)?;

    let commands = if let Some(env_config) = &config.env {
        if !env_config.hash.is_empty() {
            vec![
                format!("cd {} && docker load -i {}-latest.tar", config.name, config.name),
                format!(
                    "cd {} && export SOPS_AGE_KEY={} && sops exec-env encrypted.env 'docker compose -p hobby up -d'",
                    config.name,
                    secret_key
                ),
                format!("cd {} && rm {}-latest.tar", config.name, config.name),
            ]
        } else {
            vec![
                format!(
                    "cd {} && docker load -i {}-latest.tar",
                    config.name, config.name
                ),
                format!("cd {} && docker compose -p hobby up -d", config.name),
                format!("cd {} && rm {}-latest.tar", config.name, config.name),
            ]
        }
    } else {
        vec![
            format!(
                "cd {} && docker load -i {}-latest.tar",
                config.name, config.name
            ),
            format!("cd {} && docker compose -p hobby up -d", config.name),
            format!("cd {} && rm {}-latest.tar", config.name, config.name),
        ]
    };

    run_ssh_commands(
        &session,
        &commands.iter().map(|s| s.as_str()).collect::<Vec<&str>>(),
    )?;

    info!("Application deployed successfully");
    Ok(())
}

fn transfer_compose_file(config: &AppConfig) -> Result<()> {
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

fn save_application_config(config: &AppConfig) -> Result<()> {
    let data = serde_yaml::to_string(config)?;
    fs::write("./hobby.yml", data)?;
    Ok(())
}
