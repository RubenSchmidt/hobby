
# Hobby CLI

Hobby is a command-line tool designed to simplify deploying and managing hobby projects on remote servers. It automates server setup, deployment, and updates using Docker Compose and Caddy as a reverse proxy.

## Features

- 🚀 One-command server setup
- 🔒 Automatic SSL/TLS with Caddy
- 🐳 Docker-based deployments
- 🔑 Secure environment variable handling with SOPS/age
- 🔄 Rolling updates
- 🌐 Automatic domain configuration with sslip.io

## Prerequisites

- A fresh Ubuntu server with SSH access
- Root access to the server
- SSH key-based authentication configured
- Docker installed locally
- A Dockerized project with:
  - A valid `Dockerfile`
  - The application listening on a single port
  - (Optional) `.env` file for environment variables

### Example Project Structure
```
my-project/
├── Dockerfile
├── src/
├── .env (optional)
└── hobby.yml <- Hobby configuration generated after running `hobby setup`
```

### Minimal Dockerfile Example
```dockerfile
FROM node:18-alpine
WORKDIR /app
COPY . .
RUN npm install
EXPOSE 8080
CMD ["npm", "start"]
```


## Installation

### Option 1: Download Binary (Recommended)

1. Download the latest binary for your platform from the [releases page](https://github.com/yourusername/hobby/releases)

2. Make it executable and move it to your PATH:

```bash
# Linux/macOS
chmod +x hobby
sudo mv hobby /usr/local/bin/
```

### Option 2: Build from Source

If you have Rust installed and want to build from source:

```bash
# Clone and build
git clone https://github.com/yourusername/hobby
cd hobby
cargo install --path .
```


## Quick Start

0. Buy yourself an Ubuntu VM / VPS.

I usually do this from [Hetzner](https://www.hetzner.com/) and pick the cheapest one to start out with. Add your personal SSH key to the server, Hobby CLI will use that to log in as root the first time.


2. Initialize your server:
```bash
hobby setup your-server-ip
```

2. Configure your application:
Edit `hobby.yml` in your project directory:
```yaml
name: "myapp" <- Change to your application name
version: "V0"
port: 8080 <- Change to your application port
server: your-server-ip
url: myapp.your-server-ip.sslip.io <- Change your application URL if needed
```

3. Launch your application:
```bash
hobby launch
```

4. Deploy updates:
```bash
hobby deploy
```

## Configuration

### Basic Configuration (hobby.yml)
```yaml
name: "myapp"           # Your application name
version: "V0"           # Version tag (automatically incremented)
port: 8080             # Application port
server: 1.2.3.4        # Server IP address
url: myapp.example.com # Application URL
```

### Environment Variables
```yaml
# hobby.yml
name: "myapp"
# ...
env:
  file: .env  # Path to your environment file
```

## Commands

- `hobby setup <server-ip>`: Initialize server with Docker, Caddy, and security configurations
- `hobby launch`: First-time deployment of your application
- `hobby deploy`: Deploy updates to your application

## How It Works

1. **Setup**: Configures server with:
   - Secure hobby user
   - Docker and Docker Compose
   - Caddy reverse proxy
   - SOPS/age encryption for secrets

2. **Launch**: First deployment:
   - Builds Docker image
   - Creates Docker Compose configuration
   - Sets up Caddy routing
   - Configures SSL/TLS

3. **Deploy**: Updates existing deployment:
   - Builds new image
   - Updates containers with zero downtime
   - Maintains persistent volumes
   - Updates environment variables if changed

## Security Features

- Automatic SSL/TLS certificates
- Environment variable encryption
- Non-root user deployment
- SSH hardening

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

[MIT License](LICENSE)
