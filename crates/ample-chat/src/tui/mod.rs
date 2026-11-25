pub mod app;
pub mod terminal;
pub mod ui;

pub use app::App;
pub use terminal::{init_terminal, restore_terminal};

use std::{sync::Arc, time::Duration};

use crossterm::event::{Event, EventStream};
use futures_util::StreamExt;
use tokio::sync::mpsc;

use crate::client::{authenticate, spawn_ws_client, ClientEvent};

/// Runs the client TUI application from start to finish.
pub async fn run_client_tui(
    host: &str,
    port: u16,
    username: &str,
    password: &str,
    use_tls: bool,
) -> anyhow::Result<()> {
    let scheme_http = if use_tls { "https" } else { "http" };
    let scheme_ws = if use_tls { "wss" } else { "ws" };

    let base_url = format!("{scheme_http}://{host}:{port}");
    let ws_url = format!("{scheme_ws}://{host}:{port}");

    if !use_tls {
        eprintln!();
        eprintln!("⚠️  ============================================================== ⚠️");
        eprintln!("⚠️  WARNING: Connecting to ample-chat WITHOUT TLS (plaintext).   ⚠️");
        eprintln!("⚠️  Messages are end-to-end encrypted, but transport metadata,   ⚠️");
        eprintln!("⚠️  usernames, and traffic patterns are visible on the network.  ⚠️");
        eprintln!("⚠️  Use --tls when connecting across untrusted networks.         ⚠️");
        eprintln!("⚠️  ============================================================== ⚠️");
        eprintln!();
        // Give user a brief moment to read the warning before terminal switch
        tokio::time::sleep(Duration::from_millis(700)).await;
    }

    eprintln!("Starting zero-knowledge SRP-6a handshake with {base_url}...");
    let http_client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()?;

    let session = match authenticate(&http_client, &base_url, username, password).await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Authentication error: {e}");
            return Err(anyhow::anyhow!("Authentication failed: {e}"));
        }
    };

    eprintln!(
        "✓ SRP authenticated successfully (User ID: {})",
        &session.user_id[..8]
    );

    // Setup terminal
    let mut terminal = init_terminal()?;
    let mut app = App::new(username, use_tls);

    let (outgoing_tx, outgoing_rx) = mpsc::channel::<String>(128);
    let (event_tx, mut event_rx) = mpsc::channel::<ClientEvent>(128);

    let session_arc = Arc::new(session);
    let ws_handle = spawn_ws_client(ws_url, session_arc, outgoing_rx, event_tx);

    let mut event_stream = EventStream::new();

    loop {
        terminal.draw(|f| ui::render(f, &app))?;

        if app.should_quit {
            break;
        }

        tokio::select! {
            // User terminal input
            maybe_event = event_stream.next() => {
                match maybe_event {
                    Some(Ok(Event::Key(key_event))) => {
                        if let Some(msg_to_send) = app.handle_key(key_event) {
                            let _ = outgoing_tx.send(msg_to_send).await;
                        }
                    }
                    Some(Ok(Event::Resize(_, _))) => {
                        // Terminal resized, will redraw on next iteration
                    }
                    _ => {}
                }
            }

            // Client network event
            client_event = event_rx.recv() => {
                match client_event {
                    Some(ClientEvent::Connected) => {
                        app.connected = true;
                        app.status = "Connected".to_string();
                    }
                    Some(ClientEvent::Disconnected(reason)) => {
                        app.connected = false;
                        app.status = format!("Disconnected ({reason})");
                    }
                    Some(ClientEvent::MessageReceived(msg)) => {
                        app.add_message(msg);
                    }
                    Some(ClientEvent::UserJoined(user)) => {
                        app.add_user(user);
                    }
                    Some(ClientEvent::UserLeft(user_id)) => {
                        app.remove_user(&user_id);
                    }
                    Some(ClientEvent::UserList(users)) => {
                        app.set_user_list(users);
                    }
                    Some(ClientEvent::Notice(notice)) => {
                        app.status = notice;
                    }
                    None => {
                        break;
                    }
                }
            }
        }
    }

    // Cleanup: restore terminal mode
    restore_terminal(&mut terminal)?;
    ws_handle.abort();
    eprintln!("\nDisconnected from ample-chat. Keys wiped from RAM.");
    Ok(())
}
