use crate::crypto::cipher::NONCE_SIZE;
use rand::{rngs::OsRng, RngCore};

/// Generates a cryptographically secure 12-byte random nonce using the operating system's CSPRNG.
pub fn generate_nonce() -> [u8; NONCE_SIZE] {
    let mut nonce = [0u8; NONCE_SIZE];
    OsRng.fill_bytes(&mut nonce);
    nonce
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn test_nonce_uniqueness() {
        let mut set = HashSet::new();
        for _ in 0..10_000 {
            let nonce = generate_nonce();
            assert!(
                set.insert(nonce),
                "nonce collision detected: nonce was reused"
            );
        }
    }

    #[test]
    fn test_nonce_not_all_zeros() {
        let nonce = generate_nonce();
        assert_ne!(nonce, [0u8; NONCE_SIZE]);
    }
}
