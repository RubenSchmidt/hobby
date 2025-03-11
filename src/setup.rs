use age::secrecy::ExposeSecret;
use anyhow::Result;
use homedir::my_home;
use ssh2::Session;
use std::fs;
use std::path::PathBuf;
use tracing::info;

use crate::commands::{connect_ssh, run_ssh_commands};

pub fn setup(server_addr: String) -> Result<()> {
    let root_session = connect_ssh("root", &server_addr)?;
    setup_hobby_user(&root_session)?;
    let ssh_session = connect_ssh("hobby", &server_addr)?;
    configure_server(&ssh_session)?;

    let config_dir = create_config_directory()?;

    create_age_keys_if_not_exist(&config_dir)?;

    save_default_config(&server_addr)?;
    print_success_message();
    Ok(())
}

fn create_config_directory() -> Result<PathBuf> {
    let config_dir = my_home()?
        .ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?
        .join(".config/hobby");

    fs::create_dir_all(&config_dir)?;
    Ok(config_dir)
}

fn save_default_config(server_addr: &str) -> Result<()> {
    let default_url = format!("my-app.{}.sslip.io", server_addr);
    let conf = format!(
        r#"
name: "myapp"
version: "V0"
port: 8080
server: {}
url: {}
#env:
#  file: .env
#  hash: ""
#volumes:
#  - dbdata:/app/db/
"#,
        server_addr, default_url
    );

    fs::write("hobby.yml", conf.as_bytes())
        .map_err(|e| anyhow::anyhow!("Failed to write config file: {}", e))?;

    Ok(())
}

fn create_age_keys_if_not_exist(config_dir: &PathBuf) -> Result<()> {
    let public_key_path = config_dir.join("key.pub");
    let secret_key_path = config_dir.join("key.txt");

    // Check if keys already exist
    if public_key_path.exists() && secret_key_path.exists() {
        info!("Age keys already exist, skipping generation");
        return Ok(());
    }

    info!("Generating new age keys...");
    let sk = age::x25519::Identity::generate();
    let pk = sk.to_public();

    let binding = sk.to_string();
    let sk_str = binding.expose_secret();

    fs::write(&public_key_path, pk.to_string())?;
    fs::write(&secret_key_path, sk_str)?;

    info!("Age keys generated successfully");
    Ok(())
}

fn setup_hobby_user(session: &Session) -> Result<()> {
    info!("Setting up hobby user...");

    let commands = vec![
        "id hobby || useradd -m -s /bin/bash -G sudo hobby",
        "echo \"hobby ALL=(ALL) NOPASSWD: ALL\" >> /etc/sudoers.d/hobby",
        "mkdir -p /home/hobby/.ssh/",
        "cat /root/.ssh/authorized_keys | tee -a /home/hobby/.ssh/authorized_keys",
        "chown hobby:hobby /home/hobby/.ssh/authorized_keys",
        "chmod 600 /home/hobby/.ssh/authorized_keys",
    ];

    run_ssh_commands(session, &commands)?;
    info!("Hobby user setup completed successfully");
    Ok(())
}

fn configure_server(session: &Session) -> Result<()> {
    info!("Configuring server...");

    info!("Setting up basic system...");
    setup_basic_system(session)?;
    info!("Setting up Docker...");
    setup_docker(session)?;
    info!("Setting up Caddy...");
    setup_caddy(session)?;

    info!("Server configuration completed successfully");
    Ok(())
}

fn setup_basic_system(session: &Session) -> Result<()> {
    let commands = vec![
        "sudo sed -i 's/#PermitRootLogin prohibit-password/PermitRootLogin no/' /etc/ssh/sshd_config && sudo systemctl restart ssh",
        "sudo apt-get update -y",
        "sudo apt-get upgrade -y",
        "sudo apt-get install age ca-certificates curl vim -y",
        "ARCH=$(dpkg --print-architecture)",
        "if [ \"$ARCH\" = \"arm64\" ]; then SOPS_ARCH=arm64; else SOPS_ARCH=amd64; fi",
        "curl -LO \"https://github.com/getsops/sops/releases/download/v3.9.1/sops-v3.9.1.linux.$SOPS_ARCH\"",
        "sudo mv \"sops-v3.9.1.linux.$SOPS_ARCH\" /usr/local/bin/sops",
        "sudo chmod +x /usr/local/bin/sops",
    ];
    run_ssh_commands(session, &commands)
}

fn setup_docker(session: &Session) -> Result<()> {
    let commands = vec![
    	"sudo apt-get update -y",
    	"sudo install -m 0755 -d /etc/apt/keyrings",
    	"sudo curl -fsSL https://download.docker.com/linux/ubuntu/gpg -o /etc/apt/keyrings/docker.asc",
    	"sudo chmod a+r /etc/apt/keyrings/docker.asc",
    	"echo \"deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.asc] https://download.docker.com/linux/ubuntu $(. /etc/os-release && echo \"$VERSION_CODENAME\") stable\" | sudo tee /etc/apt/sources.list.d/docker.list > /dev/null",
    	"sudo apt-get update -y",
    	"sudo apt-get install docker-ce docker-ce-cli containerd.io docker-buildx-plugin docker-compose-plugin -y",
    	"sudo usermod -aG docker hobby",
	];
    run_ssh_commands(session, &commands)
}

fn setup_caddy(session: &Session) -> Result<()> {
    run_ssh_commands(session, &["mkdir caddy"])?;

    let docker_compose_content = r#"
version: "3.7"
services:
  caddy:
    image: lucaslorentz/caddy-docker-proxy:ci-alpine
    ports:
      - 80:80
      - 443:443
    environment:
      - CADDY_INGRESS_NETWORKS=caddy
    networks:
      - caddy
    volumes:
      - /var/run/docker.sock:/var/run/docker.sock
      - caddy_data:/data
    restart: unless-stopped

networks:
  caddy:
    external: true

volumes:
  caddy_data: {}
"#;

    let docker_compose_command = format!(
        "cd caddy && echo '{}' >> docker-compose.caddy.yml",
        docker_compose_content.replace("'", r"'\''")
    );

    let commands = vec![
        "sudo docker network create caddy",
        &docker_compose_command,
        "cd caddy && sudo docker compose -p hobby -f docker-compose.caddy.yml up -d",
    ];

    // Run the commands
    run_ssh_commands(session, &commands)?;

    info!("Caddy setup completed successfully");
    Ok(())
}

fn print_success_message() {
    info!("Initialization completed successfully");
    info!("Age keys saved to ~/.config/hobby");
    info!("Default app configuration saved to ./hobby.yaml");
    info!("Make sure to update the app configuration before deploying");
}
