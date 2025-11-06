use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Request body for creating an item
#[derive(Serialize, Deserialize, ToSchema)]
pub struct CreateItemRequest {
    pub name: String,
    pub description: String,
}

/// Response for creating an item
#[derive(Serialize, Deserialize, ToSchema)]
pub struct CreateItemResponse {
    pub id: u32,
    pub name: String,
    pub description: String,
}