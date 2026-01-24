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
#[utoipa::path(
    get,
    path = "/api/v1/{org_id}/documents/{doc_id}/export",
    tag = "documents",
    responses(
        (status = 200, description = "Document exported successfully", body = DocumentExportResponse)
    ),
    params(
        ("org_id" = String, Path, description = "Organization ID"),
        ("doc_id" = String, Path, description = "Document ID")
    )
)]
#[allow(dead_code)]
pub async fn doc_export_doc() {}

#[derive(OpenApi)]
#[openapi(
    paths(
        health_check_doc,
        ready_check_doc,
        diagnostics_doc,
        doc_export_doc,
    ),
    components(
        schemas(HealthResponse, ReadyResponse, DiagnosticsResponse, DocumentExportResponse)
    ),
    tags(
        (name = "health", description = "Health check endpoints"),
        (name = "diagnostics", description = "Diagnostics endpoints"),
        (name = "documents", description = "Document management endpoints")
    )
)]
pub struct ApiDoc;
