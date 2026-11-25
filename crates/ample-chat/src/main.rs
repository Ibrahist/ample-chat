#![forbid(unsafe_code)]

use ample_chat::cli::{resolve_password, Cli, Commands};
use ample_chat::tui::run_client_tui;
use ample_chat_server::ServerConfig;
use clap::Parser;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Tracing defaults to WARN level to avoid logging sensitive data, credentials, or plaintext
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn"));

    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer())
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Serve {
            host,
            port,
            password,
            cert,
            key,
        } => {
            let pwd = resolve_password(password, "Enter room password for server: ")?;
            let mut config = ServerConfig::new(host, port, &*pwd);
            config.cert_path = cert;
            config.key_path = key;

            ample_chat_server::run_server(config, None).await?;
        }
        Commands::Connect {
            host,
            port,
            username,
            password,
            tls,
        } => {
            let pwd = resolve_password(password, &format!("Enter room password for {username}: "))?;
            run_client_tui(&host, port, &username, &pwd, tls).await?;
        }
    }

    Ok(())
}
