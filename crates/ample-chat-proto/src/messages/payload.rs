use crate::error::CodecError;
use chrono::Utc;
use serde::{Deserialize, Serialize};

/// Inner payload encrypted by sender before transmission.
///
/// Security: Includes sender username and a monotonic per-sender counter
/// to prevent spoofing and relay replays.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EncryptedPayload {
    pub sender: String,
    pub counter: u64,
    pub timestamp_ms: i64,
    pub content: String,
}

impl EncryptedPayload {
    pub fn new(sender: impl Into<String>, counter: u64, content: impl Into<String>) -> Self {
        Self {
            sender: sender.into(),
            counter,
            timestamp_ms: Utc::now().timestamp_millis(),
            content: content.into(),
        }
    }

    pub fn to_json_bytes(&self) -> Result<Vec<u8>, CodecError> {
        serde_json::to_vec(self).map_err(|e| CodecError::Json(e.to_string()))
    }

    pub fn from_json_bytes(bytes: &[u8]) -> Result<Self, CodecError> {
        serde_json::from_slice(self_or_trim(bytes)).map_err(|e| CodecError::Json(e.to_string()))
    }
}

fn self_or_trim(bytes: &[u8]) -> &[u8] {
    let mut b = bytes;
    while let Some((&first, rest)) = b.split_first() {
        if first.is_ascii_whitespace() {
            b = rest;
        } else {
            break;
        }
    }
    while let Some((&last, rest)) = b.split_last() {
        if last.is_ascii_whitespace() {
            b = rest;
        } else {
            break;
        }
    }
    b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_payload_round_trip() {
        let payload = EncryptedPayload::new("alice", 42, "Secret message content");
        let bytes = payload.to_json_bytes().expect("serialize payload");
        let decoded = EncryptedPayload::from_json_bytes(&bytes).expect("deserialize payload");
        assert_eq!(payload, decoded);
    }
}
