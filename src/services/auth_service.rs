use tracing::info;
use axum::http::{self};
use crate::ws::userctx;
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation, TokenData};

// Get the auth token from a request
pub fn get_auth_token<B>(req: &http::Request<B>) -> Result<String, String> {
    // 1. Try to get token from Authorization header
    if let Some(auth_header) = req.headers().get(http::header::AUTHORIZATION) {
        let auth_str = auth_header.to_str().map_err(|_| "Invalid Authorization header".to_string())?;
        Ok(auth_str
            .strip_prefix("Bearer ")
            .unwrap_or(auth_str)
            .to_string())
    }
    // 2. Try to get token from cookies
    else {
        let cookie_header = req.headers().get(http::header::COOKIE)
            .ok_or_else(|| "Missing Authorization header or Cookie".to_string())?
            .to_str()
            .map_err(|_| "Invalid Cookie header".to_string())?;
        
        for cookie in cookie::Cookie::split_parse(cookie_header) {
            if let Ok(c) = cookie {
                if c.name() == "auth_token" {
                    return Ok(c.value().to_string());
                }
            }
        }
        Err("auth_token cookie not found".to_string())
    }
}

// Get the user principals from a JWT token
pub fn get_user_prpls(token: &str, force_refresh: bool) -> Result<(String, Vec<String>), String> {
   
    // Validate the auth_token as a JWT token
    let config = crate::config::get_config();
    if let Some(secret) = &config.cloud_auth_jwt_secret {
        match validate_jwt(token, secret) {

            // When a valid token is found, get the UID
            Ok(token_data) => {
                if let Some(uid) = token_data.claims.get("sub").and_then(|v| v.as_str()) {
                    info!("JWT token validated successfully for user: {}", uid);

                    // Get roles from the token claims
                    let roles = match token_data.claims.get("roles").and_then(|v| v.as_array()) {
                        Some(roles_array) => roles_array.iter().filter_map(|r| r.as_str().map(|s| s.to_string())).collect::<Vec<String>>(),
                        None => Vec::new(),
                    };

                    // When we have the UID, fetch the user context
                    return match userctx::get_or_fetch_user_ctx_blocking(uid, roles, force_refresh) {
                        Ok(user_ctx) => {
                            // Get all the principals for the user
                            let prpls = user_ctx.get_all_prpls();
                            return Ok((uid.to_string(), prpls));
                        }
                        Err(e) => {
                            Err(format!("Failed to load user context for {}: {}", uid, e))
                        }
                    };
                } else {
                    Err(format!("Can't extract a UID from the JWT token"))
                }
            },
            Err(e) => {
                Err(format!("JWT validation failed: {}", e))
            }
        }
    } else {
        Err(format!("No JWT secret configured!"))
    }
}

// Validate a JWT token and return the token data
pub fn validate_jwt(token: &str, secret: &str) -> Result<TokenData<serde_json::Value>, jsonwebtoken::errors::Error> {
    let validation = Validation::new(Algorithm::HS256);
    let decoding_key = DecodingKey::from_secret(secret.as_bytes());
    decode::<serde_json::Value>(token, &decoding_key, &validation)
}