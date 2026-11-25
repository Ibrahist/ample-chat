use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};

use ample_chat_proto::ErrorResponse;

use crate::state::AppState;

#[derive(Deserialize)]
pub struct ClearQuery {
    pub user_id: Option<String>,
}

#[derive(Serialize)]
pub struct ClearResponse {
    pub status: String,
}

pub async fn clear_handler(
    State(state): State<AppState>,
    Query(query): Query<ClearQuery>,
) -> Response {
    let user_id = match query.user_id {
        Some(id) if !id.trim().is_empty() => id,
        _ => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse::new("user_id parameter required")),
            )
                .into_response();
        }
    };

    if !state.authenticated_sessions.contains_key(&user_id) {
        return (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse::new("Unauthorized session")),
        )
            .into_response();
    }

    state.messages.write().await.clear();
    (
        StatusCode::OK,
        Json(ClearResponse {
            status: "cleared".to_string(),
        }),
    )
        .into_response()
}
