

use axum::{http::StatusCode, Json};
use uuid::Uuid;
use crate::models::ErrorResponse;

const CLOUD_ADMIN_PRPL: &str = "r/Colabri-CloudAdmin";

pub fn _is_authenticated(prpls: &Vec<String>) -> bool {
    !prpls.is_empty()
}

pub fn is_cloud_admin(prpls: &Vec<String>) -> bool {
    prpls.iter().any(|p| p == CLOUD_ADMIN_PRPL)
}

pub fn _is_service(prpls: &Vec<String>, service_name: &str) -> bool {
    let service_prpl = format!("s/{}", service_name);
    prpls.iter().any(|p| p == &service_prpl)
}

pub fn _is_org_admin(prpls: &Vec<String>, org_id: &str) -> bool {

    if is_cloud_admin(prpls) {
        return true;
    }

    let org_prefix = format!("{}/f/admin", org_id);
    prpls.iter().any(|p| p == &org_prefix)
}

pub fn _is_org_member(prpls: &Vec<String>, org_id: &str) -> bool {

    if is_cloud_admin(prpls) {
        return true;
    }

    let org_prefix = format!("{}/u/", org_id);
    prpls.iter().any(|p| p.starts_with(&org_prefix))
}

pub fn ensure_service(prpls: &Vec<String>, service_name: &str) -> Result<String, (StatusCode, Json<ErrorResponse>)> {
    
    let service_prpl = format!("s/{}", service_name);
    if prpls.iter().any(|p| p == &service_prpl) {
        return Ok(service_prpl);
    }

    if is_cloud_admin(prpls) {
        return Ok(CLOUD_ADMIN_PRPL.to_string());
    }

    let status = StatusCode::FORBIDDEN;
    Err((status, Json(ErrorResponse {
        code: status.as_u16(),
        status: status.to_string(),
        error: format!("Service '{}' access denied", service_name),
    })))
}

pub fn _ensure_org_member(prpls: &Vec<String>, org_id: &str) -> Result<(Option<Uuid>, String), (StatusCode, Json<ErrorResponse>)> {
    let org_prefix = format!("{}/u/", org_id);
    if let Some(p) = prpls.iter().find(|p| p.starts_with(&org_prefix)) {
        let uuid_str: String = p.strip_prefix(&org_prefix).unwrap().to_string();
        if let Ok(uuid) = Uuid::parse_str(&uuid_str) {
            return Ok((Some(uuid), p.to_string()));
        }
    }

    if is_cloud_admin(prpls) {
        return Ok((None, CLOUD_ADMIN_PRPL.to_string()));
    }

    let status = StatusCode::FORBIDDEN;
    Err((status, Json(ErrorResponse {
        code: status.as_u16(),
        status: status.to_string(),
        error: "User is not a member of the organization".to_string(),
    })))
}

pub fn _ensure_service_or_org_member(prpls: &Vec<String>, service_name: &str, org_id: &str) -> Result<(Option<Uuid>, String), (StatusCode, Json<ErrorResponse>)> {
    let service_prpl = format!("s/{}", service_name);
    if prpls.iter().any(|p| p == &service_prpl) {
        return Ok((None, service_prpl));
    }

    let org_prefix = format!("{}/u/", org_id);
    if let Some(p) = prpls.iter().find(|p| p.starts_with(&org_prefix)) {
        let uuid_str: String = p.strip_prefix(&org_prefix).unwrap().to_string();
        if let Ok(uuid) = Uuid::parse_str(&uuid_str) {
            return Ok((Some(uuid), p.to_string()));
        }
    }

    if is_cloud_admin(prpls) {
        return Ok((None, CLOUD_ADMIN_PRPL.to_string()));
    }

    let status = StatusCode::FORBIDDEN;
    Err((status, Json(ErrorResponse {
        code: status.as_u16(),
        status: status.to_string(),
        error: "Access denied".to_string(),
    })))
}

pub fn ensure_cloud_admin(prpls: &Vec<String>) -> Result<String, (StatusCode, Json<ErrorResponse>)> {
    if is_cloud_admin(prpls) {
        return Ok(CLOUD_ADMIN_PRPL.to_string());
    }

    let status = StatusCode::FORBIDDEN;
    Err((status, Json(ErrorResponse {
        code: status.as_u16(),
        status: status.to_string(),
        error: "Cloud Admin access required".to_string(),
    })))
}

