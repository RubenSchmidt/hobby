use crate::commands::{connect_ssh, run_ssh_commands};
use crate::config::{load_app_config, load_secret_key, validate_environment, AppConfig};
use crate::docker::build_and_transfer_image;
use crate::env;
use anyhow::Result;
use std::fs;
use std::time::Instant;
use tracing::info;

pub fn deploy() -> Result<()> {
    let start = Instant::now();

    validate_environment()?;

    let mut app_config = load_app_config()?;

    if let Some(env_config) = &app_config.env {
        if !env_config.file.is_empty() {
            info!("Checking environment file changes...");
            env::encrypt_and_upload_env_file(&mut app_config)?;
        }
    }
    info!("Building and transferring docker image...");
    build_and_transfer_image(&app_config)?;

    info!("Deploying application...");
    deploy_application(&app_config)?;

    update_version_and_config(&mut app_config)?;

    info!(
        "Deployment completed successfully in {:?}",
        start.elapsed().as_secs()
    );
    info!("Application available at: {}", app_config.url);
    Ok(())
}

fn deploy_application(config: &AppConfig) -> Result<()> {
    let session = connect_ssh("hobby", &config.server)?;

    let mut commands = vec![format!(
        "cd {} && docker load -i {}-latest.tar",
        config.name, config.name
    )];

    info!("Deploying application...");
    let deploy_command = if let Some(env_config) = &config.env {
        if !env_config.file.is_empty() {
            let secret_key = load_secret_key()?;
            format!(
                "cd {} && export SOPS_AGE_KEY={} && sops exec-env encrypted.env 'docker compose -p hobby up -d'",
                config.name, secret_key
            )
        } else {
            format!("cd {} && docker compose -p hobby up -d", config.name)
        }
    } else {
        format!("cd {} && docker compose -p hobby up -d", config.name)
    };

    commands.push(deploy_command.clone());
    commands.push(format!(
        "cd {} && rm {}-latest.tar",
        config.name, config.name
    ));
    let commands: Vec<&str> = commands.iter().map(|s| s.as_str()).collect();
    run_ssh_commands(&session, &commands)?;
    Ok(())
}

fn update_version_and_config(config: &mut AppConfig) -> Result<()> {
    let version = config.version.trim_start_matches('V');
    let version_int = version.parse::<i64>()?;
    config.version = format!("V{}", version_int + 1);

    let config_yaml = serde_yaml::to_string(&config)?;
    fs::write("./hobby.yml", config_yaml)?;
    Ok(())
}
