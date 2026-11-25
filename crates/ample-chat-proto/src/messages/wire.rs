use crate::crypto::cipher::{NONCE_SIZE, TAG_SIZE};
use crate::error::CodecError;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};

pub const CURRENT_VERSION: u8 = 1;
pub const MIN_WIRE_FRAME_SIZE: usize = 1 + NONCE_SIZE + TAG_SIZE; // 29 bytes

/// Wire frame for encrypted messages:
/// Layout: `[1 byte version][12 bytes nonce][AES-256-GCM ciphertext + 16-byte auth tag]`
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WireFrame {
    pub version: u8,
    pub nonce: [u8; NONCE_SIZE],
    pub ciphertext: Vec<u8>,
}

impl WireFrame {
    pub fn new(nonce: [u8; NONCE_SIZE], ciphertext: Vec<u8>) -> Self {
        Self {
            version: CURRENT_VERSION,
            nonce,
            ciphertext,
        }
    }

    /// Serializes the wire frame to binary representation.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(1 + NONCE_SIZE + self.ciphertext.len());
        bytes.push(self.version);
        bytes.extend_from_slice(&self.nonce);
        bytes.extend_from_slice(&self.ciphertext);
        bytes
    }

    /// Parses binary representation into a WireFrame.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, CodecError> {
        if bytes.len() < MIN_WIRE_FRAME_SIZE {
            return Err(CodecError::FrameTooShort {
                min: MIN_WIRE_FRAME_SIZE,
                got: bytes.len(),
            });
        }

        let version = bytes[0];
        if version != CURRENT_VERSION {
            return Err(CodecError::UnsupportedVersion {
                expected: CURRENT_VERSION,
                got: version,
            });
        }

        let mut nonce = [0u8; NONCE_SIZE];
        nonce.copy_from_slice(&bytes[1..1 + NONCE_SIZE]);

        let ciphertext = bytes[1 + NONCE_SIZE..].to_vec();

        Ok(Self {
            version,
            nonce,
            ciphertext,
        })
    }

    /// Encodes the wire frame to a standard base64 string for transmission.
    pub fn to_base64(&self) -> String {
        BASE64.encode(self.to_bytes())
    }

    /// Decodes a WireFrame from a base64 string.
    pub fn from_base64(s: &str) -> Result<Self, CodecError> {
        let raw = BASE64
            .decode(s.trim())
            .map_err(|e| CodecError::Base64(e.to_string()))?;
        Self::from_bytes(&raw)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn test_wire_frame_round_trip() {
        let nonce = [3u8; NONCE_SIZE];
        let ciphertext = vec![0xaa; TAG_SIZE + 10];
        let frame = WireFrame::new(nonce, ciphertext);

        let bytes = frame.to_bytes();
        let decoded = WireFrame::from_bytes(&bytes).expect("decode wire frame");
        assert_eq!(frame, decoded);

        let b64 = frame.to_base64();
        let from_b64 = WireFrame::from_base64(&b64).expect("decode from base64");
        assert_eq!(frame, from_b64);
    }

    #[test]
    fn test_wire_frame_too_short() {
        let short = vec![1u8; MIN_WIRE_FRAME_SIZE - 1];
        let err = WireFrame::from_bytes(&short);
        assert!(matches!(err, Err(CodecError::FrameTooShort { .. })));
    }

    #[test]
    fn test_wire_frame_invalid_version() {
        let mut bytes = vec![99u8; MIN_WIRE_FRAME_SIZE];
        bytes[0] = 99; // invalid version
        let err = WireFrame::from_bytes(&bytes);
        assert!(matches!(err, Err(CodecError::UnsupportedVersion { .. })));
    }

    // Property-based test using proptest
    proptest! {
        #[test]
        fn proptest_wire_frame_roundtrip(
            nonce in prop::array::uniform12(any::<u8>()),
            ciphertext in prop::collection::vec(any::<u8>(), TAG_SIZE..1024)
        ) {
            let frame = WireFrame::new(nonce, ciphertext);
            let bytes = frame.to_bytes();
            let parsed = WireFrame::from_bytes(&bytes).unwrap();
            prop_assert_eq!(frame, parsed);
        }

        #[test]
        fn proptest_invalid_bytes_fail_safely(
            random_bytes in prop::collection::vec(any::<u8>(), 0..28)
        ) {
            // Any bytes shorter than 29 must fail gracefully without panic
            let result = WireFrame::from_bytes(&random_bytes);
            assert!(result.is_err());
        }
    }
}
