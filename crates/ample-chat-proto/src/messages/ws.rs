use serde::{Deserialize, Serialize};

/// WebSocket envelope sent by the server to connected clients.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ServerWsMessage {
    #[serde(rename = "init")]
    Init {
        messages: Vec<MessageRecord>,
        users: Vec<UserSummary>,
    },
    #[serde(rename = "message")]
    Message { data: MessageRecord },
    #[serde(rename = "user_joined")]
    UserJoined { user_id: String, username: String },
    #[serde(rename = "user_left")]
    UserLeft { user_id: String, username: String },
}

/// A single message record in the server backlog or broadcast.
/// Note: `text` contains the base64-encoded WireFrame (ciphertext).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageRecord {
    pub id: String,
    pub text: String,
    pub timestamp: String,
    pub username: String,
    pub user_ip: String,
}

/// Summary of an active user connected to the room.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserSummary {
    pub user_id: String,
    pub username: String,
}
