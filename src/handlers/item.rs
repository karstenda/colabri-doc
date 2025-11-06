use axum::{Json, http::StatusCode};
use crate::models::{CreateItemRequest, CreateItemResponse};

/// Create a new item
pub async fn create_item(
    Json(payload): Json<CreateItemRequest>,
) -> (StatusCode, Json<CreateItemResponse>) {
    (
        StatusCode::CREATED,
        Json(CreateItemResponse {
            id: 1,
            name: payload.name,
            description: payload.description,
        }),
    )
}