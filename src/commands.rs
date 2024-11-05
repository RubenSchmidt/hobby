use anyhow::Result;
use ssh2::Session;
use std::io::Read;
use tracing::info;

pub fn connect_ssh(user: &str, host: &str) -> Result<Session> {
    info!("Connecting to SSH server...");
    let tcp = std::net::TcpStream::connect(format!("{}:22", host))?;
    let mut session = Session::new()?;
    session.set_tcp_stream(tcp);
    session.handshake()?;
    session.userauth_agent(user)?;
    Ok(session)
}

pub fn run_ssh_commands(session: &Session, commands: &[&str]) -> Result<()> {
    for cmd in commands {
        let mut channel = session.channel_session()?;
        channel.exec(cmd)?;

        // Read all output from the channel
        let mut output = String::new();
        channel.read_to_string(&mut output)?;

        // Read stderr as well
        let mut stderr = String::new();
        channel.stderr().read_to_string(&mut stderr)?;

        // Now wait for the channel to close
        channel.wait_close()?;

        let exit_status = channel.exit_status()?;
        if exit_status != 0 {
            anyhow::bail!(
                "Command failed: {}\nOutput: {}\nError: {}",
                cmd,
                output,
                stderr
            );
        }
    }
    Ok(())
}
