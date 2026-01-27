use moka::sync::Cache;
use serde_json::Value;
use std::sync::OnceLock;
use std::time::Duration;
use tokio::runtime::Handle;
use tracing::{error, info};

use crate::clients::app_service_client;

#[derive(Clone, Debug)]
pub struct UserCtx {
    pub principals: Vec<String>,
    pub token_roles: Vec<String>,
}

impl UserCtx {
    pub fn get_user_principal(&self, org_id: &str) -> Option<String> {
        let org_prefix = format!("{}/u/", org_id);
        self.principals
            .iter()
            .find(|principal| principal.starts_with(&org_prefix))
            .cloned()
    }

    pub fn get_all_prpls(&self) -> Vec<String> {
        // Get the calculated principals for the user
        let mut all_prpls = self.principals.clone();
        // Get the roles from the token and add them as principals
        for prpl in &self.token_roles {
            all_prpls.push("r/".to_string() + prpl);
        }
        all_prpls
    }
}

static USER_CTX_CACHE: OnceLock<Cache<String, UserCtx>> = OnceLock::new();

pub fn init_user_ctx_cache() {
    USER_CTX_CACHE.get_or_init(|| {
        Cache::builder()
            .max_capacity(100_000)
            .time_to_idle(Duration::from_secs(5 * 60))
            .build()
    });
    info!("User cache initialized");
}
pub fn get_user_ctx_cache() -> &'static Cache<String, UserCtx> {
    USER_CTX_CACHE
        .get()
        .expect("User cache not initialized. Call init_user_ctx_cache() first.")
}

fn parse_principals_from_json(prpls_json: Value) -> Vec<String> {
    if let Some(prpls_val) = prpls_json.get("prpls") {
        serde_json::from_value(prpls_val.clone()).unwrap_or_else(|e| {
            error!("Failed to parse principals array from 'prpls' field: {}", e);
            Vec::new()
        })
    } else {
        serde_json::from_value(prpls_json).unwrap_or_else(|e| {
            error!("Failed to parse principals JSON: {}", e);
            Vec::new()
        })
    }
}

async fn fetch_user_prpls_from_service(uid: &str) -> Result<Vec<String>, String> {
    let client = app_service_client::get_app_service_client()
        .ok_or_else(|| "App service client not initialized".to_string())?;

    let prpls_json = client
        .get_prpls(uid)
        .await
        .map_err(|e| {
            error!("Failed to retrieve principals for user {}: {}", uid, e);
            format!("Failed to retrieve principals: {}", e)
        })?;

    info!("Retrieved principals for user {}: {}", uid, prpls_json);
    let principals = parse_principals_from_json(prpls_json);
    Ok(principals)
}

pub fn get_user_ctx_from_cache(uid: &str) -> Option<UserCtx> {
    let cache = get_user_ctx_cache();
    cache.get(uid)
}

pub async fn get_or_fetch_user_ctx_async(uid: &str, token_roles: Vec<String>) -> Result<UserCtx, String> {
    let cache = get_user_ctx_cache();

    if let Some(ctx) = cache.get(uid) {
        return Ok(ctx);
    }

    info!("User context cache miss for uid {}. Refreshing from app service.", uid);
    let fetched_prpls = fetch_user_prpls_from_service(uid).await?;
    
    // Create a new user context and insert it into the cache
    let new_ctx = UserCtx {
        principals: fetched_prpls,
        token_roles: token_roles,
    };
    cache.insert(uid.to_string(), new_ctx.clone());
    
    // Return the newly created user context
    Ok(new_ctx)
}

pub fn get_or_fetch_user_ctx_blocking(uid: &str, token_roles: Vec<String>) -> Result<UserCtx, String> {
    let uid_owned = uid.to_string();

    tokio::task::block_in_place(move || {
        Handle::current().block_on(async move {
            get_or_fetch_user_ctx_async(&uid_owned, token_roles).await
        })
    })
}