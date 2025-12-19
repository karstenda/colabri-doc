use reqwest::Client;
use tokio::sync::OnceCell;
use std::sync::Arc;
use jsonwebtoken::{encode, Header, EncodingKey};
use serde::{Serialize, Deserialize};
use chrono::{Utc, Duration};

static APP_SERVICE_CLIENT: OnceCell<Arc<AppServiceClient>> = OnceCell::const_new();

#[derive(Debug)]
pub struct AppServiceClient {
    client: Client,
    base_url: String,
    jwt_secret: String,
    service_name: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,
    #[serde(rename = "type")]
    type_: String,
    exp: usize,
}

impl AppServiceClient {
    pub fn new(base_url: String, jwt_secret: String, service_name: String) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .expect("Failed to build reqwest client");
            
        Self {
            client,
            base_url,
            jwt_secret,
            service_name,
        }
    }

    fn generate_token(&self) -> String {
        let expiration = Utc::now()
            .checked_add_signed(Duration::seconds(60)) // 1 minute expiration
            .expect("valid timestamp")
            .timestamp();

        let claims = Claims {
            sub: self.service_name.clone(),
            type_: "service".to_string(),
            exp: expiration as usize,
        };

        encode(&Header::default(), &claims, &EncodingKey::from_secret(self.jwt_secret.as_bytes()))
            .expect("Failed to generate JWT")
    }

    /// Example method to make a request to the app service
    pub async fn get_prpls(&self, uid: &str) -> Result<serde_json::Value, reqwest::Error> {
        let token = self.generate_token();
        let url = format!("{}/auth/prpls/{}", self.base_url, uid);
        self.client.get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send().await?.json().await
    }
    
    // Add more methods here as needed
}

/// Initialize the global AppServiceClient
pub fn init_app_service_client(base_url: String, jwt_secret: String, service_name: String) -> Result<(), &'static str> {
    let client = AppServiceClient::new(base_url, jwt_secret, service_name);
    APP_SERVICE_CLIENT.set(Arc::new(client))
        .map_err(|_| "AppServiceClient already initialized")
}

/// Get the global AppServiceClient instance
pub fn get_app_service_client() -> Option<Arc<AppServiceClient>> {
    APP_SERVICE_CLIENT.get().cloned()
}
