use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use std::time::Instant;

use ample_chat_proto::{
    ErrorResponse, SrpInitRequest, SrpInitResponse, SrpVerifyRequest, SrpVerifyResponse,
};

use crate::state::{generate_uuid, AppState, AuthenticatedSession, PendingSrpSession};

/// Handler for POST /srp/init
pub async fn srp_init_handler(
    State(state): State<AppState>,
    Json(payload): Json<SrpInitRequest>,
) -> Response {
    if payload.username.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new("Username cannot be empty")),
        )
            .into_response();
    }

    if state.is_username_online(&payload.username) {
        return (
            StatusCode::CONFLICT,
            Json(ErrorResponse::new("Username is already taken")),
        )
            .into_response();
    }

    let client_a = match BASE64.decode(payload.a.trim()) {
        Ok(bytes) => bytes,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new("Invalid base64 ephemeral A")),
            )
                .into_response();
        }
    };

    let handshake = match state.srp_server.init_handshake(&client_a) {
        Ok(hs) => hs,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new(format!("SRP init failed: {e}"))),
            )
                .into_response();
        }
    };

    let user_id = generate_uuid();
    let b_b64 = BASE64.encode(&handshake.b_pub);
    let salt_b64 = BASE64.encode(state.srp_server.salt());
    let room_salt_b64 = BASE64.encode(state.room_salt);

    state.pending_srp.insert(
        user_id.clone(),
        PendingSrpSession {
            username: payload.username,
            handshake,
            created_at: Instant::now(),
        },
    );

    (
        StatusCode::OK,
        Json(SrpInitResponse {
            user_id,
            b: b_b64,
            salt: salt_b64,
            room_salt: room_salt_b64,
        }),
    )
        .into_response()
}

/// Handler for POST /srp/verify
pub async fn srp_verify_handler(
    State(state): State<AppState>,
    Json(payload): Json<SrpVerifyRequest>,
) -> Response {
    let (_, pending) = match state.pending_srp.remove(&payload.user_id) {
        Some(entry) => entry,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse::new("Invalid or expired session")),
            )
                .into_response();
        }
    };

    let client_m = match BASE64.decode(payload.m.trim()) {
        Ok(bytes) => bytes,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new("Invalid base64 client proof M")),
            )
                .into_response();
        }
    };

    let (server_proof, session_key) = match pending.handshake.verify_client_proof(&client_m) {
        Ok(res) => res,
        Err(_) => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse::new("SRP authentication failed")),
            )
                .into_response();
        }
    };

    let h_amk_b64 = BASE64.encode(&server_proof);
    let session_key_b64 = BASE64.encode(session_key.as_slice());

    state.authenticated_sessions.insert(
        payload.user_id.clone(),
        AuthenticatedSession {
            user_id: payload.user_id,
            username: pending.username,
            session_key,
            created_at: Instant::now(),
            last_active: Instant::now(),
        },
    );

    (
        StatusCode::OK,
        Json(SrpVerifyResponse {
            h_amk: h_amk_b64,
            session_key: session_key_b64,
        }),
    )
        .into_response()
}
