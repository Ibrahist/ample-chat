#![forbid(unsafe_code)]

//! `ample-chat-server` is an encrypted, RAM-only chat server.
//!
//! Features:
//! - All users, sessions, and messages kept strictly in memory.
//! - Zero-knowledge SRP-6a authentication.
//! - Ciphertext broadcast over WebSockets without server inspection capability.
//! - Per-connection rate limiting, message size enforcement, and session timeouts.
//! - Optional TLS encryption.

pub mod config;
pub mod error;
pub mod rate_limit;
pub mod routes;
pub mod state;
pub mod tls;

pub use config::ServerConfig;
pub use error::ServerError;
pub use state::AppState;

use axum::{
    routing::{delete, get, post},
    Router,
};
use routes::{
    clear_handler, health_handler, srp_init_handler, srp_verify_handler, ws_chat_handler,
};
use std::{net::SocketAddr, time::Duration};
use tokio::sync::oneshot;
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::trace::TraceLayer;

/// Builds the axum application router with all routes and layers.
pub fn create_app(state: AppState) -> Router {
    Router::new()
        .route("/srp/init", post(srp_init_handler))
        .route("/srp/verify", post(srp_verify_handler))
        .route("/ws/chat", get(ws_chat_handler))
        .route("/health", get(health_handler))
        .route("/clear", delete(clear_handler))
        .layer(RequestBodyLimitLayer::new(state.max_message_size + 4096))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

/// Runs the server until Ctrl+C, SIGTERM, or an optional oneshot shutdown signal is received.
pub async fn run_server(
    config: ServerConfig,
    shutdown_rx: Option<oneshot::Receiver<()>>,
) -> Result<(), ServerError> {
    if !config.is_tls() {
        eprintln!();
        eprintln!("⚠️  ============================================================== ⚠️");
        eprintln!("⚠️  WARNING: Running ample-chat server WITHOUT TLS (plaintext).  ⚠️");
        eprintln!("⚠️  E2E message content is encrypted, but transport metadata,    ⚠️");
        eprintln!("⚠️  traffic timing, and room salts are exposed on the network.   ⚠️");
        eprintln!("⚠️  For production usage, pass --cert <cert.pem> and --key <key>  ⚠️");
        eprintln!("⚠️  ============================================================== ⚠️");
        eprintln!();
        tracing::warn!("Server running without TLS - transport layer unencrypted");
    }

    let addr: SocketAddr = format!("{}:{}", config.host, config.port)
        .parse()
        .map_err(|e| ServerError::Config(format!("Invalid host/port: {e}")))?;

    tracing::info!("Listening on {}", addr);
    eprintln!("Server listening on http://{addr}");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    run_server_with_listener(listener, config, shutdown_rx).await
}

/// Runs the server on an existing bound TcpListener until shutdown signal or Ctrl+C.
pub async fn run_server_with_listener(
    listener: tokio::net::TcpListener,
    config: ServerConfig,
    shutdown_rx: Option<oneshot::Receiver<()>>,
) -> Result<(), ServerError> {
    let state = AppState::new(
        &config.password,
        config.max_message_size,
        config.rate_limit_per_second,
        config.session_timeout_secs,
    );

    // Spawn background task for stale session cleanup
    let cleanup_state = state.clone();
    let cleanup_handle = tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(60));
        loop {
            interval.tick().await;
            cleanup_state.cleanup_stale();
        }
    });

    let app = create_app(state);
    let make_service = app.into_make_service_with_connect_info::<SocketAddr>();
    let server_fut = axum::serve(listener, make_service);

    let shutdown_signal = async {
        match shutdown_rx {
            Some(rx) => {
                tokio::select! {
                    _ = rx => {
                        tracing::info!("Received test shutdown signal");
                    }
                    _ = tokio::signal::ctrl_c() => {
                        tracing::info!("Received Ctrl+C shutdown signal");
                    }
                }
            }
            None => {
                let _ = tokio::signal::ctrl_c().await;
                tracing::info!("Received Ctrl+C shutdown signal");
            }
        }
    };

    server_fut
        .with_graceful_shutdown(shutdown_signal)
        .await
        .map_err(|e| ServerError::Io(std::io::Error::other(e)))?;

    cleanup_handle.abort();
    eprintln!("Server shutdown gracefully. All RAM state wiped.");
    Ok(())
}
