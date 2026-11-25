use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use ample_chat_proto::{
    crypto::srp::{ChatSrpServer, ServerHandshake},
    MessageRecord, ServerWsMessage,
};
use chrono::Utc;
use dashmap::DashMap;
use rand::{rngs::OsRng, RngCore};
use tokio::sync::{broadcast, RwLock};
use zeroize::{Zeroize, Zeroizing};

pub const BROADCAST_CAPACITY: usize = 256;
pub const MAX_BACKLOG_MESSAGES: usize = 100;

/// Pending SRP handshake state prior to `/srp/verify`.
pub struct PendingSrpSession {
    pub username: String,
    pub handshake: ServerHandshake,
    pub created_at: Instant,
}

/// Fully authenticated session with derived SRP session key.
#[derive(Clone)]
pub struct AuthenticatedSession {
    pub user_id: String,
    pub username: String,
    pub session_key: Zeroizing<Vec<u8>>,
    pub created_at: Instant,
    pub last_active: Instant,
}

impl Drop for AuthenticatedSession {
    fn drop(&mut self) {
        // Zeroize session key on drop
        self.session_key.zeroize();
    }
}

/// Active connected WebSocket user.
#[derive(Clone, Debug)]
pub struct ActiveUser {
    pub user_id: String,
    pub username: String,
    pub connected_at: Instant,
}

/// Helper to generate UUID v4 string from OS randomness without external crate.
pub fn generate_uuid() -> String {
    let mut bytes = [0u8; 16];
    OsRng.fill_bytes(&mut bytes);
    // Set version 4
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    // Set variant RFC 4122
    bytes[8] = (bytes[8] & 0x3f) | 0x80;

    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        bytes[0], bytes[1], bytes[2], bytes[3],
        bytes[4], bytes[5],
        bytes[6], bytes[7],
        bytes[8], bytes[9],
        bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15]
    )
}

/// Thread-safe, RAM-only server state.
#[derive(Clone)]
pub struct AppState {
    pub room_salt: [u8; 16],
    pub srp_server: ChatSrpServer,
    pub pending_srp: Arc<DashMap<String, PendingSrpSession>>,
    pub authenticated_sessions: Arc<DashMap<String, AuthenticatedSession>>,
    pub active_users: Arc<DashMap<String, ActiveUser>>,
    pub messages: Arc<RwLock<Vec<MessageRecord>>>,
    pub broadcast_tx: broadcast::Sender<ServerWsMessage>,
    pub max_message_size: usize,
    pub rate_limit_per_second: u32,
    pub session_timeout: Duration,
}

impl AppState {
    pub fn new(
        password: &str,
        max_message_size: usize,
        rate_limit_per_second: u32,
        session_timeout_secs: u64,
    ) -> Self {
        let mut room_salt = [0u8; 16];
        OsRng.fill_bytes(&mut room_salt);

        let mut srp_salt = [0u8; 16];
        OsRng.fill_bytes(&mut srp_salt);

        let srp_server = ChatSrpServer::new(password, &srp_salt);
        let (broadcast_tx, _) = broadcast::channel(BROADCAST_CAPACITY);

        Self {
            room_salt,
            srp_server,
            pending_srp: Arc::new(DashMap::new()),
            authenticated_sessions: Arc::new(DashMap::new()),
            active_users: Arc::new(DashMap::new()),
            messages: Arc::new(RwLock::new(Vec::new())),
            broadcast_tx,
            max_message_size,
            rate_limit_per_second,
            session_timeout: Duration::from_secs(session_timeout_secs),
        }
    }

    /// Adds a message record to RAM backlog, retaining at most MAX_BACKLOG_MESSAGES.
    pub async fn add_message(&self, message: MessageRecord) {
        let mut msgs = self.messages.write().await;
        if msgs.len() >= MAX_BACKLOG_MESSAGES {
            msgs.remove(0);
        }
        msgs.push(message);
    }

    /// Cleans up stale pending SRP handshakes (older than 60 seconds) and expired sessions.
    pub fn cleanup_stale(&self) {
        let now = Instant::now();
        let handshake_ttl = Duration::from_secs(60);

        self.pending_srp
            .retain(|_, session| now.duration_since(session.created_at) < handshake_ttl);

        self.authenticated_sessions.retain(|user_id, session| {
            // Do not expire active WebSocket connections
            if self.active_users.contains_key(user_id) {
                return true;
            }
            now.duration_since(session.last_active) < self.session_timeout
        });
    }

    /// Checks if a username is currently taken by an active connected user.
    pub fn is_username_online(&self, username: &str) -> bool {
        self.active_users
            .iter()
            .any(|entry| entry.value().username.eq_ignore_ascii_case(username))
    }

    /// Helper to get current UTC timestamp in ISO 8601 format.
    pub fn utcnow_iso() -> String {
        Utc::now().to_rfc3339()
    }
}
