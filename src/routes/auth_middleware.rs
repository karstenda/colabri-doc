use axum::{
    extract::Request,
    http::{StatusCode},
    middleware::Next,
    response::Response,
};
use tracing::{error, info};
use crate::config;
use crate::ws::userctx;
use crate::services::auth_service::{validate_jwt, get_auth_token};

pub async fn auth_middleware(
    mut req: Request,
    next: Next,
) -> Result<Response, StatusCode> {

    // 1+2. Get the auth token from the request
    let token = match get_auth_token(&req) {
        Ok(token) => token,
        Err(_) => return Err(StatusCode::UNAUTHORIZED),
    };

    // 3. Validate Token
    let config = config::get_config();
    let secret = match &config.cloud_auth_jwt_secret {
        Some(secret) => secret,
        None => {
            error!("Cloud auth JWT secret not configured");
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };
    let token_data = match validate_jwt(&token, secret) {
        Ok(token_data) => token_data,
        Err(e) => {
            error!("JWT validation failed: {}", e);
            return Err(StatusCode::UNAUTHORIZED);
        }
    };

    // 4. Determine the type of token (user/service)
    let token_type = token_data.claims.get("type").and_then(|v| v.as_str()).ok_or_else(|| {
        error!("JWT token does not contain 'type' claim");
        StatusCode::UNAUTHORIZED
    })?;

    // 5A. If user token, extract UID and load User Context
    if token_type == "user" {   

        // Log the validation of the user token
        info!("User token validated successfully");     

        // 6A. Extract the UID
        let user_uid = if let Some(sub) = token_data.claims.get("sub").and_then(|v| v.as_str()) {
            sub.to_string()
        } else {
            error!("JWT token does not contain 'sub' claim");
            return Err(StatusCode::UNAUTHORIZED);
        };

        // 7A. Load User Context and the prpls for the user
        let user_ctx = match userctx::get_or_fetch_user_ctx_blocking(&user_uid) {
            Ok(user_ctx) => {
                user_ctx
            }
            Err(e) => {
                error!("Failed to load user context for {}: {}", user_uid, e);
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
        };
        let mut prpls = user_ctx.principals.clone();

        // 8A. Add any roles from the JWT claims to the principals
        let roles = match token_data.claims.get("roles").and_then(|v| v.as_array()) {
            Some(roles_array) => roles_array.iter().filter_map(|r| r.as_str().map(|s| s.to_string())).collect::<Vec<String>>(),
            None => Vec::new(),
        };
        for role in roles {
            let role_prpl = format!("r/{}", role);
            if !prpls.contains(&role_prpl) {
                prpls.push(role_prpl);
            }
        }

        // 9A. Set these principals into request extensions for downstream handlers
        {
            let extensions = req.extensions_mut();
            extensions.insert(prpls);
            extensions.insert(user_uid);
        }
    }
    // 5B. If this is a service token, just extract the service name as prpl
    else if token_type == "service" {

        // Log the validation of the service token
        info!("Service token validated successfully");

        // 6B. Extract the service name
        let service_name = if let Some(sub) = token_data.claims.get("sub").and_then(|v| v.as_str()) {
            sub.to_string()
        } else {
            error!("JWT token does not contain 'sub' claim");
            return Err(StatusCode::UNAUTHORIZED);
        };

        // 7B. Generate the prpls for the service
        let prpls = vec!["s/".to_string()+&service_name];

        // 8B. Set these principals into request extensions for downstream handlers
        {
            let extensions = req.extensions_mut();
            extensions.insert(prpls);
            // No user UID to insert
        }

    } else {
        error!("Invalid token type: {}", token_type);
        return Err(StatusCode::UNAUTHORIZED);
    }

    // Token is valid and we have user context, proceed to next middleware/handler
    Ok(next.run(req).await)

}
