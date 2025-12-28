use serde::{Deserialize, Serialize};
use tracing::{info, error};
use tokio::sync::OnceCell;

// Global configuration instance
static CONFIG: OnceCell<Config> = OnceCell::const_new();

/// Initialize the global configuration
pub fn init_config(config: Config) -> Result<(), &'static str> {
    CONFIG.set(config).map_err(|_| "Configuration already initialized")
}

/// Get the global configuration
pub fn get_config() -> &'static Config {
    CONFIG.get().expect("Configuration not initialized")
}

/// Application configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    /// Server host address
    #[serde(default = "default_host")]
    pub host: String,
    
    /// Server port
    #[serde(default = "default_port")]
    pub port: u16,

    /// WebSocket port
    #[serde(default = "default_websocket_port")]
    pub websocket_port: u16,

    /// Environment (dev, staging, prod)
    #[serde(default = "default_environment")]
    pub environment: String,

    /// Log level
    #[serde(default = "default_log_level")]
    pub log_level: String,

    // Cloud service identifiers
    pub cloud_pod: Option<String>,
    #[serde(default = "default_service_name")]
    pub cloud_service_name: String,
    #[serde(default = "default_service_domain")]
    pub cloud_service_domain: String,
    #[serde(default = "default_app_service_domain")]
    pub cloud_app_service_domain: String,
    #[serde(default = "default_root_service_domain")]
    pub cloud_root_domain: String,
    
    /// CORS allowed origins
    #[serde(default = "default_cors_origins")]
    pub cloud_cors_origins: String,

    /// JWT secret key
    pub cloud_auth_jwt_secret: Option<String>,

    /// GCP project ID
    pub gcp_project_id: Option<String>,

    /// Database URL
    pub db_url: Option<String>,
}

impl Config {
    /// Load configuration from environment variables or app.env file
    pub fn load() -> Result<Self, ConfigError> {
        // Try to load from app.env file first
        if std::path::Path::new("app.env").exists() {
            dotenvy::from_filename("app.env").ok();
        } else {
            // Fallback to .env file
            dotenvy::dotenv().ok();
        }
        
        // Load from environment variables using envy
        match envy::from_env::<Config>() {
            Ok(config) => {
                info!("✅ Configuration loaded successfully");
                Ok(config)
            }
            Err(e) => {
                error!("❌ Failed to load configuration: {}", e);
                Err(ConfigError::EnvError(e))
            }
        }
    }
    
    /// Get the full server address
    pub fn server_address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    /// Get the WebSocket port
    pub fn websocket_port(&self) -> u16 {
        self.websocket_port
    }

    /// Get the App Service URL
    pub fn app_service_url(&self) -> String {
        if self.environment == "development" {
            "http://localhost:8080".to_string()
        } else {
            format!("http://{}", self.cloud_app_service_domain)
        }
    }
    
}

impl Default for Config {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
            websocket_port: default_websocket_port(),
            environment: default_environment(),
            log_level: default_log_level(),
            cloud_pod: None,
            cloud_service_name: default_service_name(),
            cloud_service_domain: default_service_domain(),
            cloud_app_service_domain: default_app_service_domain(),
            cloud_root_domain: default_root_service_domain(),
            cloud_cors_origins: default_cors_origins(),
            cloud_auth_jwt_secret: None,
            gcp_project_id: None,
            db_url: None,
        }
    }
}

#[derive(Debug)]
pub enum ConfigError {
    EnvError(envy::Error),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::EnvError(e) => write!(f, "Environment variable error: {}", e),
        }
    }
}

impl std::error::Error for ConfigError {}

// Default value functions
fn default_host() -> String {
    "0.0.0.0".to_string()
}

fn default_port() -> u16 {
    3000
}

fn default_websocket_port() -> u16 {
    9001
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_service_name() -> String {
    "colabri-doc".to_string()
}

fn default_service_domain() -> String {
    "doc.colabri.cloud".to_string()
}

fn default_app_service_domain() -> String {
    "app.colabri.cloud".to_string()
}

fn default_root_service_domain() -> String {
    "colabri.cloud".to_string()
}

fn default_cors_origins() -> String {
    "http://localhost:*,http://*.colabri-local.cloud,http://colabri-local.cloud,https://*.colabri.cloud,https://colabri.cloud".to_string()
}

fn default_environment() -> String {
    "development".to_string()
}