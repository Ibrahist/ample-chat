use crate::error::CryptoError;
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use zeroize::Zeroize;

pub const KEY_SIZE: usize = 32;
pub const NONCE_SIZE: usize = 12;
pub const TAG_SIZE: usize = 16;

/// Encrypts plaintext using AES-256-GCM.
/// Returns the ciphertext with authentication tag appended.
pub fn encrypt(
    key: &[u8; KEY_SIZE],
    nonce: &[u8; NONCE_SIZE],
    plaintext: &[u8],
) -> Result<Vec<u8>, CryptoError> {
    let cipher =
        Aes256Gcm::new_from_slice(key).map_err(|e| CryptoError::EncryptionFailed(e.to_string()))?;
    let gcm_nonce = Nonce::from_slice(nonce);

    cipher
        .encrypt(gcm_nonce, plaintext)
        .map_err(|e| CryptoError::EncryptionFailed(e.to_string()))
}

/// Decrypts ciphertext using AES-256-GCM and verifies the authentication tag.
pub fn decrypt(
    key: &[u8; KEY_SIZE],
    nonce: &[u8; NONCE_SIZE],
    ciphertext: &[u8],
) -> Result<Vec<u8>, CryptoError> {
    if ciphertext.len() < TAG_SIZE {
        return Err(CryptoError::DecryptionFailed);
    }

    let cipher = Aes256Gcm::new_from_slice(key).map_err(|_| CryptoError::DecryptionFailed)?;
    let gcm_nonce = Nonce::from_slice(nonce);

    let mut decrypted = cipher
        .decrypt(gcm_nonce, ciphertext)
        .map_err(|_| CryptoError::DecryptionFailed)?;

    // Return plaintext without zeroizing immediately, caller owns it
    let result = decrypted.clone();
    decrypted.zeroize();
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_round_trip() {
        let key = [42u8; KEY_SIZE];
        let nonce = [7u8; NONCE_SIZE];
        let plaintext = b"Hello, secure ample-chat!";

        let ciphertext = encrypt(&key, &nonce, plaintext).expect("encryption succeeds");
        assert_ne!(ciphertext, plaintext);
        assert_eq!(ciphertext.len(), plaintext.len() + TAG_SIZE);

        let decrypted = decrypt(&key, &nonce, &ciphertext).expect("decryption succeeds");
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_tamper_detection() {
        let key = [42u8; KEY_SIZE];
        let nonce = [7u8; NONCE_SIZE];
        let plaintext = b"Sensitive message content";

        let mut ciphertext = encrypt(&key, &nonce, plaintext).expect("encryption succeeds");
        // Tamper with one byte
        let last = ciphertext.len() - 1;
        ciphertext[last] ^= 0x01;

        let result = decrypt(&key, &nonce, &ciphertext);
        assert!(matches!(result, Err(CryptoError::DecryptionFailed)));
    }

    #[test]
    fn test_wrong_key_fails() {
        let key1 = [42u8; KEY_SIZE];
        let key2 = [99u8; KEY_SIZE];
        let nonce = [7u8; NONCE_SIZE];
        let plaintext = b"Confidential data";

        let ciphertext = encrypt(&key1, &nonce, plaintext).expect("encryption succeeds");
        let result = decrypt(&key2, &nonce, &ciphertext);
        assert!(matches!(result, Err(CryptoError::DecryptionFailed)));
    }
}
