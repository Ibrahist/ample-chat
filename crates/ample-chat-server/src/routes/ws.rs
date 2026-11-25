use std::{net::SocketAddr, time::Instant};

use axum::{
    extract::{
        ws::{Message, WebSocket},
        ConnectInfo, Query, State, WebSocketUpgrade,
    },
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;

use ample_chat_proto::{ErrorResponse, MessageRecord, ServerWsMessage, UserSummary};

use crate::{
    rate_limit::RateLimiter,
    state::{ActiveUser, AppState},
};

#[derive(Deserialize)]
pub struct WsQuery {
    pub user_id: Option<String>,
}

pub async fn ws_chat_handler(
    State(state): State<AppState>,
    Query(query): Query<WsQuery>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    ws: WebSocketUpgrade,
) -> Response {
    let user_id = match query.user_id {
        Some(id) if !id.trim().is_empty() => id,
        _ => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse::new("user_id query parameter required")),
            )
                .into_response();
        }
    };

    let session = match state.authenticated_sessions.get(&user_id) {
        Some(s) => s.clone(),
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse::new("Invalid or expired session")),
            )
                .into_response();
        }
    };

    ws.on_upgrade(move |socket| handle_socket(socket, state, session, addr))
}

async fn handle_socket(
    socket: WebSocket,
    state: AppState,
    session: crate::state::AuthenticatedSession,
    client_addr: SocketAddr,
) {
    let user_id = session.user_id.clone();
    let username = session.username.clone();
    let user_ip = client_addr.ip().to_string();

    // Register active user
    state.active_users.insert(
        user_id.clone(),
        ActiveUser {
            user_id: user_id.clone(),
            username: username.clone(),
            connected_at: Instant::now(),
        },
    );

    let (mut sender, mut receiver) = socket.split();
    let mut broadcast_rx = state.broadcast_tx.subscribe();

    // 1. Send initial state: message backlog + active users
    let backlog = state.messages.read().await.clone();
    let users = state
        .active_users
        .iter()
        .map(|entry| UserSummary {
            user_id: entry.value().user_id.clone(),
            username: entry.value().username.clone(),
        })
        .collect::<Vec<_>>();

    let init_msg = ServerWsMessage::Init {
        messages: backlog,
        users,
    };

    if let Ok(json_str) = serde_json::to_string(&init_msg) {
        let _ = sender.send(Message::Text(json_str)).await;
    }

    // 2. Broadcast user_joined notice
    let _ = state.broadcast_tx.send(ServerWsMessage::UserJoined {
        user_id: user_id.clone(),
        username: username.clone(),
    });

    let mut rate_limiter = RateLimiter::new(
        state.rate_limit_per_second,
        state.rate_limit_per_second as f64,
    );

    // 3. Message loop
    loop {
        tokio::select! {
            // Receive from client
            msg = receiver.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        // Enforce max message size
                        if text.len() > state.max_message_size {
                            tracing::warn!("Message rejected: exceeds maximum allowed size");
                            continue;
                        }

                        // Enforce per-connection rate limit
                        if !rate_limiter.check_and_consume() {
                            tracing::warn!("Message dropped: rate limit exceeded for user");
                            continue;
                        }

                        // Update activity
                        if let Some(mut s) = state.authenticated_sessions.get_mut(&user_id) {
                            s.last_active = Instant::now();
                        }

                        let message_record = MessageRecord {
                            id: crate::state::generate_uuid(),
                            text,
                            timestamp: AppState::utcnow_iso(),
                            username: username.clone(),
                            user_ip: user_ip.clone(),
                        };

                        state.add_message(message_record.clone()).await;

                        let _ = state.broadcast_tx.send(ServerWsMessage::Message {
                            data: message_record,
                        });
                    }
                    Some(Ok(Message::Close(_))) | None => {
                        break;
                    }
                    Some(Err(_)) => {
                        break;
                    }
                    _ => {}
                }
            }

            // Receive from broadcast channel
            broadcast_msg = broadcast_rx.recv() => {
                match broadcast_msg {
                    Ok(msg) => {
                        if let Ok(json_str) = serde_json::to_string(&msg) {
                            if sender.send(Message::Text(json_str)).await.is_err() {
                                break;
                            }
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                        // Client lagged behind, continue receiving
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        break;
                    }
                }
            }
        }
    }

    // Disconnect cleanup: remove from active users and broadcast user_left
    state.active_users.remove(&user_id);
    let _ = state
        .broadcast_tx
        .send(ServerWsMessage::UserLeft { user_id, username });
}
