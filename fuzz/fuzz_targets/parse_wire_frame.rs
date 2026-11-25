#![no_main]

use ample_chat_proto::{EncryptedPayload, WireFrame};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // 1. Fuzz binary wire frame decoding against arbitrary bytes
    if let Ok(frame) = WireFrame::from_bytes(data) {
        let encoded = frame.to_bytes();
        let re_decoded =
            WireFrame::from_bytes(&encoded).expect("re-decoding valid frame must not fail");
        assert_eq!(frame, re_decoded);
    }

    // 2. Fuzz base64 wire frame decoding
    if let Ok(s) = std::str::from_utf8(data) {
        let _ = WireFrame::from_base64(s);
    }

    // 3. Fuzz JSON payload deserialization
    let _ = EncryptedPayload::from_json_bytes(data);
});
