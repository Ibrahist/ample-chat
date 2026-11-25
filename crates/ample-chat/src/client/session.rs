use std::sync::atomic::{AtomicU64, Ordering};

use ample_chat_proto::{
    crypto::{cipher::decrypt, cipher::encrypt, nonce::generate_nonce},
    EncryptedPayload, WireFrame,
};
use zeroize::{Zeroize, Zeroizing};

use super::error::ClientError;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DecryptedContent {
    Success(EncryptedPayload),
    Failed(String),
}

/// Client cryptographic session state.
pub struct ClientSession {
    pub user_id: String,
    pub username: String,
    pub room_key: Zeroizing<[u8; 32]>,
    pub session_key: Zeroizing<Vec<u8>>,
    counter: AtomicU64,
}

impl Drop for ClientSession {
    fn drop(&mut self) {
        // Explicitly wipe keys from RAM on disconnect
        self.room_key.zeroize();
        self.session_key.zeroize();
    }
}

impl ClientSession {
    pub fn new(
        user_id: impl Into<String>,
        username: impl Into<String>,
        room_key: Zeroizing<[u8; 32]>,
        session_key: Zeroizing<Vec<u8>>,
    ) -> Self {
        Self {
            user_id: user_id.into(),
            username: username.into(),
            room_key,
            session_key,
            counter: AtomicU64::new(0),
        }
    }

    /// Encrypts outgoing message text into a base64-encoded WireFrame.
    pub fn encrypt_message(&self, content: &str) -> Result<String, ClientError> {
        let counter = self.counter.fetch_add(1, Ordering::SeqCst);
        let payload = EncryptedPayload::new(&self.username, counter, content);
        let payload_bytes = payload.to_json_bytes()?;

        let nonce = generate_nonce();
        let ciphertext = encrypt(&self.room_key, &nonce, &payload_bytes)?;

        let wire_frame = WireFrame::new(nonce, ciphertext);
        Ok(wire_frame.to_base64())
    }

    /// Decrypts incoming base64-encoded WireFrame.
    pub fn decrypt_message(&self, b64_frame: &str) -> DecryptedContent {
        let frame = match WireFrame::from_base64(b64_frame) {
            Ok(f) => f,
            Err(_) => return DecryptedContent::Failed("[malformed wire frame]".to_string()),
        };

        let plaintext = match decrypt(&self.room_key, &frame.nonce, &frame.ciphertext) {
            Ok(p) => p,
            Err(_) => {
                return DecryptedContent::Failed(
                    "[decryption failed: incorrect room password or tampered ciphertext]"
                        .to_string(),
                )
            }
        };

        match EncryptedPayload::from_json_bytes(&plaintext) {
            Ok(payload) => DecryptedContent::Success(payload),
            Err(_) => DecryptedContent::Failed("[malformed payload]".to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_session(room_key_val: u8) -> ClientSession {
        ClientSession::new(
            "user-1",
            "alice",
            Zeroizing::new([room_key_val; 32]),
            Zeroizing::new(vec![room_key_val; 32]),
        )
    }

    #[test]
    fn test_session_encrypt_decrypt_round_trip() {
        let alice = dummy_session(42);
        let bob = dummy_session(42);

        let encrypted = alice.encrypt_message("Hello Bob!").expect("encrypt");
        match bob.decrypt_message(&encrypted) {
            DecryptedContent::Success(payload) => {
                assert_eq!(payload.sender, "alice");
                assert_eq!(payload.counter, 0);
                assert_eq!(payload.content, "Hello Bob!");
            }
            DecryptedContent::Failed(err) => panic!("Expected success, got {err}"),
        }

        // Alice sends another message: counter should increment to 1
        let encrypted2 = alice.encrypt_message("Second message").expect("encrypt");
        match bob.decrypt_message(&encrypted2) {
            DecryptedContent::Success(payload) => {
                assert_eq!(payload.counter, 1);
                assert_eq!(payload.content, "Second message");
            }
            DecryptedContent::Failed(err) => panic!("Expected success, got {err}"),
        }
    }

    #[test]
    fn test_session_wrong_key_fails_gracefully() {
        let alice = dummy_session(42);
        let charlie = dummy_session(99); // different room key

        let encrypted = alice.encrypt_message("Secret data").expect("encrypt");
        let result = charlie.decrypt_message(&encrypted);
        assert!(matches!(result, DecryptedContent::Failed(_)));
    }

    #[test]
    fn test_session_tampered_ciphertext_fails_gracefully() {
        let alice = dummy_session(42);
        let bob = dummy_session(42);

        let encrypted = alice.encrypt_message("Authentic message").expect("encrypt");
        // Tamper with the base64 string
        let mut tampered = encrypted.into_bytes();
        let len = tampered.len();
        tampered[len - 2] = if tampered[len - 2] == b'A' {
            b'B'
        } else {
            b'A'
        };
        let tampered_str = String::from_utf8(tampered).unwrap();

        let result = bob.decrypt_message(&tampered_str);
        assert!(matches!(result, DecryptedContent::Failed(_)));
    }
}
