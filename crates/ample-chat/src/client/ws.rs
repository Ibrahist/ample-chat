use std::{sync::Arc, time::Duration};

use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

use ample_chat_proto::{MessageRecord, ServerWsMessage, UserSummary};

use super::session::{ClientSession, DecryptedContent};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DisplayMessage {
    pub id: String,
    pub sender: String,
    pub counter: u64,
    pub timestamp: String,
    pub content: String,
    pub is_self: bool,
    pub failed: bool,
}

#[derive(Clone, Debug)]
pub enum ClientEvent {
    Connected,
    Disconnected(String),
    MessageReceived(DisplayMessage),
    UserJoined(UserSummary),
    UserLeft(String),
    UserList(Vec<UserSummary>),
    Notice(String),
}

/// Spawns the WebSocket background loop with automatic reconnect support.
pub fn spawn_ws_client(
    ws_url: String,
    session: Arc<ClientSession>,
    mut outgoing_rx: mpsc::Receiver<String>,
    event_tx: mpsc::Sender<ClientEvent>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let full_url = format!("{ws_url}/ws/chat?user_id={}", session.user_id);
        let mut retry_delay = Duration::from_secs(1);
        let max_retry_delay = Duration::from_secs(16);

        loop {
            tracing::info!("Connecting to chat WebSocket at {full_url}...");
            match connect_async(&full_url).await {
                Ok((ws_stream, _)) => {
                    retry_delay = Duration::from_secs(1);
                    let _ = event_tx.send(ClientEvent::Connected).await;

                    let (mut write, mut read) = ws_stream.split();

                    loop {
                        tokio::select! {
                            // Outgoing message to send to server
                            outgoing = outgoing_rx.recv() => {
                                match outgoing {
                                    Some(text) => {
                                        match session.encrypt_message(&text) {
                                            Ok(encrypted_b64) => {
                                                if write.send(Message::Text(encrypted_b64)).await.is_err() {
                                                    break;
                                                }
                                            }
                                            Err(e) => {
                                                let _ = event_tx.send(ClientEvent::Notice(format!("Encryption error: {e}"))).await;
                                            }
                                        }
                                    }
                                    None => {
                                        // Outgoing channel closed, exit
                                        let _ = write.send(Message::Close(None)).await;
                                        return;
                                    }
                                }
                            }

                            // Incoming message from server
                            incoming = read.next() => {
                                match incoming {
                                    Some(Ok(Message::Text(raw_json))) => {
                                        handle_server_message(&raw_json, &session, &event_tx).await;
                                    }
                                    Some(Ok(Message::Close(_))) | None => {
                                        let _ = event_tx.send(ClientEvent::Disconnected("Connection closed by server".to_string())).await;
                                        break;
                                    }
                                    Some(Err(e)) => {
                                        let _ = event_tx.send(ClientEvent::Disconnected(e.to_string())).await;
                                        break;
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    let _ = event_tx
                        .send(ClientEvent::Disconnected(format!(
                            "Failed to connect: {e}. Reconnecting in {}s...",
                            retry_delay.as_secs()
                        )))
                        .await;
                }
            }

            // Exponential backoff
            tokio::time::sleep(retry_delay).await;
            retry_delay = (retry_delay * 2).min(max_retry_delay);
        }
    })
}

async fn handle_server_message(
    raw_json: &str,
    session: &ClientSession,
    event_tx: &mpsc::Sender<ClientEvent>,
) {
    let ws_msg: ServerWsMessage = match serde_json::from_str(raw_json) {
        Ok(m) => m,
        Err(_) => return,
    };

    match ws_msg {
        ServerWsMessage::Init { messages, users } => {
            let _ = event_tx.send(ClientEvent::UserList(users)).await;
            for record in messages {
                let display = decrypt_record(&record, session);
                let _ = event_tx.send(ClientEvent::MessageReceived(display)).await;
            }
        }
        ServerWsMessage::Message { data } => {
            let display = decrypt_record(&data, session);
            let _ = event_tx.send(ClientEvent::MessageReceived(display)).await;
        }
        ServerWsMessage::UserJoined { user_id, username } => {
            let _ = event_tx
                .send(ClientEvent::UserJoined(UserSummary { user_id, username }))
                .await;
        }
        ServerWsMessage::UserLeft { user_id, .. } => {
            let _ = event_tx.send(ClientEvent::UserLeft(user_id)).await;
        }
    }
}

fn decrypt_record(record: &MessageRecord, session: &ClientSession) -> DisplayMessage {
    let is_self = record.username.eq_ignore_ascii_case(&session.username);

    match session.decrypt_message(&record.text) {
        DecryptedContent::Success(payload) => DisplayMessage {
            id: record.id.clone(),
            sender: payload.sender,
            counter: payload.counter,
            timestamp: record.timestamp.clone(),
            content: payload.content,
            is_self,
            failed: false,
        },
        DecryptedContent::Failed(err_text) => DisplayMessage {
            id: record.id.clone(),
            sender: record.username.clone(),
            counter: 0,
            timestamp: record.timestamp.clone(),
            content: err_text,
            is_self,
            failed: true,
        },
    }
}
