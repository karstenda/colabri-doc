use crate::models::{CreateItemRequest, CreateItemResponse};
use axum::{http::StatusCode, Json};

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
