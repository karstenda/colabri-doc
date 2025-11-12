use serde::{Deserialize, Serialize};
use tracing::{info, error};

/// Application configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    /// Server host address
    #[serde(default = "default_host")]
    pub host: String,
    
    /// Server port
    #[serde(default = "default_port")]
    pub port: u16,

    /// Environment (dev, staging, prod)
    #[serde(default = "default_environment")]
    pub environment: String,

    /// CORS allowed origins
    pub cors_origins: Option<String>,

    /// Log level
    #[serde(default = "default_log_level")]
    pub log_level: String,

    // Cloud service identifiers
    #[serde(default = "default_service_name")]
    pub cloud_service_name: String,
    pub cloud_pod: Option<String>,

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
    
    /// Check if running in development mode
    pub fn is_development(&self) -> bool {
        self.environment.to_lowercase() == "dev" || self.environment.to_lowercase() == "development"
    }
    
    /// Check if running in production mode
    pub fn is_production(&self) -> bool {
        self.environment.to_lowercase() == "prod" || self.environment.to_lowercase() == "production"
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
            environment: default_environment(),
            log_level: default_log_level(),
            cors_origins: None,
            cloud_service_name: default_service_name(),
            cloud_pod: None,
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

fn default_log_level() -> String {
    "info".to_string()
}

fn default_service_name() -> String {
    "colabri-doc".to_string()
}

fn default_environment() -> String {
    "development".to_string()
}