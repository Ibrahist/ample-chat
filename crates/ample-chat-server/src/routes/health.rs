use axum::{extract::State, response::Json};

use ample_chat_proto::HealthResponse;

use crate::state::AppState;

pub async fn health_handler(State(state): State<AppState>) -> Json<HealthResponse> {
    let msg_count = state.messages.read().await.len();
    let user_count = state.active_users.len();

    Json(HealthResponse {
        status: "ok".to_string(),
        messages: msg_count,
        users: user_count,
        timestamp: AppState::utcnow_iso(),
    })
}
