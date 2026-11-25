use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use reqwest::StatusCode;

use ample_chat_proto::{
    crypto::kdf::derive_room_key, crypto::srp::SrpClientSession, ErrorResponse, SrpInitRequest,
    SrpInitResponse, SrpVerifyRequest, SrpVerifyResponse,
};

use super::{error::ClientError, session::ClientSession};

/// Performs full zero-knowledge SRP-6a authentication handshake with ample-chat-server
/// and derives the shared Argon2id+HKDF room encryption key.
pub async fn authenticate(
    client: &reqwest::Client,
    base_url: &str,
    username: &str,
    password: &str,
) -> Result<ClientSession, ClientError> {
    // 1. Initialize client-side SRP state
    let client_srp = SrpClientSession::new(username, password);
    let a_b64 = BASE64.encode(&client_srp.a_pub);

    // 2. POST /srp/init { username, A }
    let init_url = format!("{base_url}/srp/init");
    let init_req = SrpInitRequest {
        username: username.to_string(),
        a: a_b64,
    };

    let init_res = client.post(&init_url).json(&init_req).send().await?;

    if init_res.status() != StatusCode::OK {
        let status = init_res.status().as_u16();
        let err_msg = match init_res.json::<ErrorResponse>().await {
            Ok(e) => e.error,
            Err(_) => "SRP init failed".to_string(),
        };
        return Err(ClientError::ServerError {
            status,
            message: err_msg,
        });
    }

    let init_data: SrpInitResponse = init_res.json().await?;

    let salt = BASE64
        .decode(&init_data.salt)
        .map_err(|e| ClientError::Auth(format!("Invalid salt base64: {e}")))?;
    let room_salt = BASE64
        .decode(&init_data.room_salt)
        .map_err(|e| ClientError::Auth(format!("Invalid room_salt base64: {e}")))?;
    let b_pub = BASE64
        .decode(&init_data.b)
        .map_err(|e| ClientError::Auth(format!("Invalid B base64: {e}")))?;

    // 3. Derive room encryption key using Argon2id + HKDF
    let room_key = derive_room_key(password, &room_salt)?;

    // 4. Client processes challenge and computes proof M
    let client_awaiting = client_srp.process_challenge(&salt, &b_pub)?;
    let m_b64 = BASE64.encode(&client_awaiting.client_proof);

    // 5. POST /srp/verify { user_id, username, M }
    let verify_url = format!("{base_url}/srp/verify");
    let verify_req = SrpVerifyRequest {
        user_id: init_data.user_id.clone(),
        username: username.to_string(),
        m: m_b64,
    };

    let verify_res = client.post(&verify_url).json(&verify_req).send().await?;

    if verify_res.status() != StatusCode::OK {
        let status = verify_res.status().as_u16();
        let err_msg = match verify_res.json::<ErrorResponse>().await {
            Ok(e) => e.error,
            Err(_) => "SRP verify failed".to_string(),
        };
        return Err(ClientError::ServerError {
            status,
            message: err_msg,
        });
    }

    let verify_data: SrpVerifyResponse = verify_res.json().await?;

    let h_amk = BASE64
        .decode(&verify_data.h_amk)
        .map_err(|e| ClientError::Auth(format!("Invalid H_AMK base64: {e}")))?;

    // 6. Verify server proof H_AMK
    let session_key = client_awaiting.verify_server(&h_amk)?;

    Ok(ClientSession::new(
        init_data.user_id,
        username,
        room_key,
        session_key,
    ))
}
