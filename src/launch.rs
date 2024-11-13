use crate::commands::{connect_ssh, run_ssh_commands};
use crate::config::{self, load_app_config, load_secret_key, validate_environment, AppConfig};
use crate::docker;
use crate::env;
use anyhow::Result;
use std::fs;
use tracing::info;

pub fn launch() -> Result<()> {
    let start = std::time::Instant::now();

    validate_environment()?;

    info!("Loading app config...");
    let mut app_config = load_app_config()?;

    let secret_key = load_secret_key()?;

    let compose = docker::build_compose_config(&app_config)?;

    docker::write_docker_compose_file(&compose)?;

    deploy_application(&mut app_config, &secret_key)?;

    config::save_application_config(&app_config)?;

    fs::remove_file("docker-compose.yaml")?;

    info!(
        "Application launched successfully in {:?}",
        start.elapsed().as_secs()
    );
    info!("Application available at: {}", app_config.url);
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
    docker::build_and_transfer_image(config)?;

    info!("Transferring docker-compose file...");
    docker::transfer_compose_file(config)?;

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
