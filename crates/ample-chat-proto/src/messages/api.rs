use serde::{Deserialize, Serialize};

/// Request sent to `POST /srp/init`
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SrpInitRequest {
    pub username: String,
    #[serde(rename = "A")]
    pub a: String,
}

/// Response returned from `POST /srp/init`
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SrpInitResponse {
    pub user_id: String,
    #[serde(rename = "B")]
    pub b: String,
    pub salt: String,
    pub room_salt: String,
}

/// Request sent to `POST /srp/verify`
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SrpVerifyRequest {
    pub user_id: String,
    pub username: String,
    #[serde(rename = "M")]
    pub m: String,
}

/// Response returned from `POST /srp/verify`
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SrpVerifyResponse {
    #[serde(rename = "H_AMK")]
    pub h_amk: String,
    pub session_key: String,
}

/// Response returned from `GET /health`
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub messages: usize,
    pub users: usize,
    pub timestamp: String,
}

/// Standard error response JSON
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
}

impl ErrorResponse {
    pub fn new(msg: impl Into<String>) -> Self {
        Self { error: msg.into() }
    }
}
