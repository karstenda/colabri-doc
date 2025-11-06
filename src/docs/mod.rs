use utoipa::OpenApi;
use crate::models::*;

/// Health check endpoint
#[utoipa::path(
    get,
    path = "/api/health",
    responses(
        (status = 200, description = "Service is healthy", body = HealthResponse)
    )
)]
#[allow(dead_code)]
pub async fn health_check_doc() {}

/// Create a new item
#[utoipa::path(
    post,
    path = "/api/items",
    request_body = CreateItemRequest,
    responses(
        (status = 201, description = "Item created successfully", body = CreateItemResponse)
    )
)]
#[allow(dead_code)]
pub async fn create_item_doc() {}

#[derive(OpenApi)]
#[openapi(
    paths(
        health_check_doc,
        create_item_doc,
    ),
    components(
        schemas(HealthResponse, CreateItemRequest, CreateItemResponse)
    ),
    tags(
        (name = "api", description = "API endpoints")
    )
)]
pub struct ApiDoc;