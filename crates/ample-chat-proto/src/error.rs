use thiserror::Error;

#[derive(Error, Debug)]
pub enum CryptoError {
    #[error("Argon2id key derivation failed: {0}")]
    Argon2(String),
    #[error("HKDF expansion failed")]
    HkdfExpansionFailed,
    #[error("Encryption failed: {0}")]
    EncryptionFailed(String),
    #[error("Decryption failed: authentication tag mismatch or invalid ciphertext")]
    DecryptionFailed,
    #[error("Invalid key length: expected {expected}, got {got}")]
    InvalidKeyLength { expected: usize, got: usize },
    #[error("Invalid nonce length: expected {expected}, got {got}")]
    InvalidNonceLength { expected: usize, got: usize },
}

#[derive(Error, Debug, PartialEq, Eq)]
pub enum SrpError {
    #[error("SRP authentication failed")]
    AuthFailed,
    #[error("Invalid public ephemeral parameter: {0}")]
    InvalidParameter(String),
    #[error("Server proof verification failed")]
    ServerVerificationFailed,
    #[error("Client proof verification failed")]
    ClientVerificationFailed,
    #[error("Base64 decode error: {0}")]
    Base64(String),
}

#[derive(Error, Debug, PartialEq, Eq)]
pub enum CodecError {
    #[error("Wire frame too short: minimum {min} bytes, got {got}")]
    FrameTooShort { min: usize, got: usize },
    #[error("Unsupported protocol version: expected {expected}, got {got}")]
    UnsupportedVersion { expected: u8, got: u8 },
    #[error("JSON serialization/deserialization failed: {0}")]
    Json(String),
    #[error("Base64 decode error: {0}")]
    Base64(String),
    #[error("UTF-8 decoding failed")]
    Utf8,
}

#[derive(Error, Debug)]
pub enum ProtocolError {
    #[error(transparent)]
    Crypto(#[from] CryptoError),
    #[error(transparent)]
    Srp(#[from] SrpError),
    #[error(transparent)]
    Codec(#[from] CodecError),
}
