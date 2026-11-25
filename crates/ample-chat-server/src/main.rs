#![forbid(unsafe_code)]

use ample_chat_server::{run_server, ServerConfig};
use std::env;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Tracing defaults to WARN level to avoid logging sensitive data or flooding
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn"));

    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer())
        .init();

    let host = env::var("AMPLE_CHAT_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = env::var("AMPLE_CHAT_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8000);

    let password = env::var("AMPLE_CHAT_PASSWORD").unwrap_or_else(|_| {
        eprintln!("Error: AMPLE_CHAT_PASSWORD environment variable is required");
        std::process::exit(1);
    });

    let config = ServerConfig::new(host, port, password);
    run_server(config, None).await?;
    Ok(())
}
