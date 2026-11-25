use clap::{Parser, Subcommand};
use std::path::PathBuf;
use zeroize::Zeroizing;

#[derive(Parser, Debug)]
#[command(
    name = "ample-chat",
    version,
    about = "Encrypted, RAM-only terminal chat with zero-knowledge SRP-6a authentication and E2E encryption"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Run the ample-chat relay server
    Serve {
        /// Host address to bind to (e.g. 0.0.0.0 or 127.0.0.1)
        host: String,

        /// Port to bind to (e.g. 8000)
        port: u16,

        /// Room password. If omitted, read from AMPLE_CHAT_PASSWORD or prompted securely via stdin.
        #[arg(short, long)]
        password: Option<String>,

        /// Optional path to TLS certificate PEM file
        #[arg(long)]
        cert: Option<PathBuf>,

        /// Optional path to TLS private key PEM file
        #[arg(long)]
        key: Option<PathBuf>,
    },

    /// Connect to an ample-chat server and join the room
    Connect {
        /// Server IP address or hostname
        host: String,

        /// Server port
        port: u16,

        /// Your chat display username
        username: String,

        /// Room password. If omitted, read from AMPLE_CHAT_PASSWORD or prompted securely via stdin.
        #[arg(short, long)]
        password: Option<String>,

        /// Connect using HTTPS / WSS (TLS)
        #[arg(long)]
        tls: bool,
    },
}

/// Resolves the password securely from CLI argument, environment variable, or stdin prompt.
pub fn resolve_password(
    cli_password: Option<String>,
    prompt_label: &str,
) -> std::io::Result<Zeroizing<String>> {
    // 1. CLI argument
    if let Some(pw) = cli_password {
        if !pw.is_empty() {
            return Ok(Zeroizing::new(pw));
        }
    }

    // 2. Environment variable AMPLE_CHAT_PASSWORD
    if let Ok(env_pw) = std::env::var("AMPLE_CHAT_PASSWORD") {
        if !env_pw.is_empty() {
            return Ok(Zeroizing::new(env_pw));
        }
    }

    // 3. Interactive stdin prompt
    let prompted = rpassword::prompt_password(prompt_label)?;
    if prompted.trim().is_empty() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Password cannot be empty",
        ));
    }

    Ok(Zeroizing::new(prompted))
}
