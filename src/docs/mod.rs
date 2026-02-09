use crate::models::*;
use utoipa::OpenApi;

/// Health check endpoint
#[utoipa::path(
    get,
    path = "/api/health",
    tag = "health",
    responses(
        (status = 200, description = "Service is healthy", body = HealthResponse)
    )
)]
#[allow(dead_code)]
pub async fn health_check_doc() {}

/// Health check endpoint
#[utoipa::path(
    get,
    path = "/api/ready",
    tag = "health",
    responses(
        (status = 200, description = "Service is ready", body = ReadyResponse)
    )
)]
#[allow(dead_code)]
pub async fn ready_check_doc() {}

/// Get diagnostics for the server
#[utoipa::path(
    get,
    path = "/api/v1/diagnostics",
    tag = "diagnostics",
    responses(
        (status = 200, description = "Server diagnostics retrieved successfully", body = DiagnosticsResponse)
    )
)]
#[allow(dead_code)]
pub async fn diagnostics_doc() {}

/// Export a document
/// 
/// This endpoint will always return the latest state of a document.
#[utoipa::path(
    get,
    path = "/api/v1/{org_id}/documents/{doc_id}",
    tag = "documents",
    responses(
        (status = 200, description = "Latest document state retrieved successfully", body = DocumentLatestResponse)
    ),
    params(
        ("org_id" = String, Path, description = "Organization ID"),
        ("doc_id" = String, Path, description = "Document ID"),
        ("format" = Option<String>, Query, description = "Output format: json, binary, or both (default: json)")
    )
)]
#[allow(dead_code)]
pub async fn doc_latest_doc() {}

/// Export a document
/// 
/// This endpoint will return the state of a document at a specific point in time determined by the version parameters. Since the version vector can be large, this is a POST endpoint that accepts the version parameters in the request body. It does however never modify the state of the document.
#[utoipa::path(
    post,
    path = "/api/v1/{org_id}/documents/{doc_id}/version",
    tag = "documents",
    request_body(content = DocumentVersionRequest, description = "Version request parameters"),
    responses(
        (status = 200, description = "Document version state retrieved successfully", body = DocumentVersionResponse)
    ),
    params(
        ("org_id" = String, Path, description = "Organization ID"),
        ("doc_id" = String, Path, description = "Document ID")
    )
)]
#[allow(dead_code)]
pub async fn doc_version_doc() {}


/// Delete a document
/// 
/// This endpoint will delete a document. It is used when a user wants to remove a document from their personal space or a shared library.
#[utoipa::path(
    delete,
    path = "/api/v1/{org_id}/documents/{doc_id}",
    tag = "documents",
    responses(
        (status = 200, description = "Document deleted successfully", body = DocumentDeleteResponse)
    ),
    params(
        ("org_id" = String, Path, description = "Organization ID"),
        ("doc_id" = String, Path, description = "Document ID")
    )
)]
#[allow(dead_code)]
pub async fn doc_delete_doc() {}

/// Move a document to a library
/// 
/// This endpoint moves a document to a library. It is used when a user wants to move a document from their personal space to a shared library.
#[utoipa::path(
    post,
    path = "/api/v1/{org_id}/documents/{doc_id}/move-lib",
    tag = "documents",
    request_body(content = DocumentMoveLibRequest, description = "Move to library request parameters"),
    responses(
        (status = 200, description = "Document moved successfully", body = DocumentMoveLibResponse)
    ),
    params(
        ("org_id" = String, Path, description = "Organization ID"),
        ("doc_id" = String, Path, description = "Document ID")
    )
)]
#[allow(dead_code)]
pub async fn doc_move_lib_doc() {}

#[derive(OpenApi)]
#[openapi(
    paths(
        health_check_doc,
        ready_check_doc,
        diagnostics_doc,
        doc_latest_doc,
        doc_version_doc,
        doc_delete_doc,
        doc_move_lib_doc,
    ),
    components(
        schemas(HealthResponse, 
            ReadyResponse, 
            DiagnosticsResponse, 
            DocumentLatestResponse, 
            DocumentVersionRequest, 
            DocumentVersionResponse,
            DocumentDeleteResponse,
            DocumentMoveLibRequest,
            DocumentMoveLibResponse,
            ErrorResponse)
    ),
    tags(
        (name = "health", description = "Health check endpoints"),
        (name = "diagnostics", description = "Diagnostics endpoints"),
        (name = "documents", description = "Document management endpoints")
    )
)]
pub struct ApiDoc;
