#![forbid(unsafe_code)]

//! `ample-chat-proto` provides shared protocol definitions, SRP-6a authentication,
//! and cryptographic helpers for `ample-chat`.

pub mod crypto;
pub mod error;
pub mod messages;

pub use error::{CodecError, CryptoError, ProtocolError, SrpError};
pub use messages::{
    EncryptedPayload, ErrorResponse, HealthResponse, MessageRecord, ServerWsMessage,
    SrpInitRequest, SrpInitResponse, SrpVerifyRequest, SrpVerifyResponse, UserSummary, WireFrame,
};
