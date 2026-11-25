use crate::error::CryptoError;
use argon2::Argon2;
use hkdf::Hkdf;
use sha2::Sha256;
use zeroize::Zeroizing;

const ROOM_KEY_INFO: &[u8] = b"ample-chat-room-key-v1";

/// Derives the 32-byte room encryption key from the room password and server-provided room_salt.
///
/// Security: The original cmd-chat derived room_key = HKDF(password, room_salt), which makes
/// password cracking fast for an observer with ciphertext and room_salt.
/// ample-chat applies Argon2id first to provide memory-hard, compute-intensive password stretching,
/// followed by HKDF-SHA256 expansion to produce the final 256-bit AES-GCM room key.
pub fn derive_room_key(
    password: &str,
    room_salt: &[u8],
) -> Result<Zeroizing<[u8; 32]>, CryptoError> {
    if room_salt.is_empty() {
        return Err(CryptoError::Argon2(
            "room_salt must not be empty".to_string(),
        ));
    }

    // Step 1: Memory-hard password hashing using Argon2id
    let mut argon2_out = Zeroizing::new([0u8; 32]);
    let argon2 = Argon2::default();
    argon2
        .hash_password_into(password.as_bytes(), room_salt, argon2_out.as_mut())
        .map_err(|e| CryptoError::Argon2(e.to_string()))?;

    // Step 2: HKDF-SHA256 expansion with domain separation
    let hkdf = Hkdf::<Sha256>::new(Some(room_salt), argon2_out.as_ref());
    let mut room_key = Zeroizing::new([0u8; 32]);
    hkdf.expand(ROOM_KEY_INFO, room_key.as_mut())
        .map_err(|_| CryptoError::HkdfExpansionFailed)?;

    Ok(room_key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_derive_room_key_deterministic() {
        let password = "super-secret-password";
        let room_salt = [1u8; 16];

        let key1 = derive_room_key(password, &room_salt).expect("derive key 1");
        let key2 = derive_room_key(password, &room_salt).expect("derive key 2");
        assert_eq!(*key1, *key2);
    }

    #[test]
    fn test_different_salt_produces_different_key() {
        let password = "super-secret-password";
        let salt1 = [1u8; 16];
        let salt2 = [2u8; 16];

        let key1 = derive_room_key(password, &salt1).expect("derive key 1");
        let key2 = derive_room_key(password, &salt2).expect("derive key 2");
        assert_ne!(*key1, *key2);
    }

    #[test]
    fn test_different_password_produces_different_key() {
        let salt = [1u8; 16];
        let key1 = derive_room_key("password123", &salt).expect("derive key 1");
        let key2 = derive_room_key("password124", &salt).expect("derive key 2");
        assert_ne!(*key1, *key2);
    }

    #[test]
    fn test_empty_salt_rejected() {
        let result = derive_room_key("password", &[]);
        assert!(result.is_err());
    }
}
