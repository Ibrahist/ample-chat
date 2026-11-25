use ample_chat_proto::{
    crypto::srp::SrpClientSession, HealthResponse, SrpInitRequest, SrpInitResponse,
    SrpVerifyRequest, SrpVerifyResponse,
};
use ample_chat_server::{create_app, AppState};
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use tower::ServiceExt;

fn test_state() -> AppState {
    AppState::new("test-room-password", 65536, 10, 3600)
}

#[tokio::test]
async fn test_health_endpoint() {
    let state = test_state();
    let app = create_app(state);

    let req = Request::builder()
        .uri("/health")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let body = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let health: HealthResponse = serde_json::from_slice(&body).unwrap();
    assert_eq!(health.status, "ok");
    assert_eq!(health.users, 0);
}

#[tokio::test]
async fn test_srp_init_missing_or_empty_username() {
    let state = test_state();
    let app = create_app(state);

    let req = Request::builder()
        .uri("/srp/init")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(r#"{"username": "", "A": "AQID"}"#))
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_srp_verify_nonexistent_session() {
    let state = test_state();
    let app = create_app(state);

    let req = Request::builder()
        .uri("/srp/verify")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(
            r#"{"user_id": "nonexistent", "username": "alice", "M": "AQID"}"#,
        ))
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_full_srp_handshake_flow() {
    let password = "test-room-password";
    let state = test_state();
    let app = create_app(state);

    let client_session = SrpClientSession::new("alice", password);
    let a_b64 = BASE64.encode(&client_session.a_pub);

    // 1. POST /srp/init
    let init_req_json = serde_json::to_vec(&SrpInitRequest {
        username: "alice".to_string(),
        a: a_b64,
    })
    .unwrap();

    let req = Request::builder()
        .uri("/srp/init")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(init_req_json))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let body = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let init_resp: SrpInitResponse = serde_json::from_slice(&body).unwrap();
    assert!(!init_resp.user_id.is_empty());

    let salt = BASE64.decode(&init_resp.salt).unwrap();
    let b_pub = BASE64.decode(&init_resp.b).unwrap();

    // 2. Client processes challenge
    let client_awaiting = client_session.process_challenge(&salt, &b_pub).unwrap();
    let m_b64 = BASE64.encode(&client_awaiting.client_proof);

    // 3. POST /srp/verify
    let verify_req_json = serde_json::to_vec(&SrpVerifyRequest {
        user_id: init_resp.user_id.clone(),
        username: "alice".to_string(),
        m: m_b64,
    })
    .unwrap();

    let req = Request::builder()
        .uri("/srp/verify")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(verify_req_json))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);

    let body = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    let verify_resp: SrpVerifyResponse = serde_json::from_slice(&body).unwrap();

    let h_amk = BASE64.decode(&verify_resp.h_amk).unwrap();
    let server_session_key = BASE64.decode(&verify_resp.session_key).unwrap();

    // 4. Client verifies server proof
    let client_session_key = client_awaiting.verify_server(&h_amk).unwrap();
    assert_eq!(*client_session_key, server_session_key);

    // 5. DELETE /clear with authenticated user_id succeeds
    let clear_req = Request::builder()
        .uri(format!("/clear?user_id={}", init_resp.user_id))
        .method("DELETE")
        .body(Body::empty())
        .unwrap();

    let clear_res = app.oneshot(clear_req).await.unwrap();
    assert_eq!(clear_res.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_clear_unauthorized_fails() {
    let state = test_state();
    let app = create_app(state);

    let req = Request::builder()
        .uri("/clear?user_id=fake-id")
        .method("DELETE")
        .body(Body::empty())
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}
