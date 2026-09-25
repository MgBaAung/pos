use crate::error::{AppError, AppResult};
use config::{Config, Environment, File};
use serde::Deserialize;
use std::env;

/// Fallback JWT secret used only in development; rejected in production.
const INSECURE_DEFAULT_JWT_SECRET: &str = "default-secret-key-change-in-production-32chars";
/// Marker for the development database password; rejected in production.
const INSECURE_DEFAULT_DB_PASSWORD: &str = "postgres:password@";

#[derive(Debug, Deserialize, Clone)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Deserialize, Clone)]
pub struct JwtConfig {
    pub secret: String,
    pub expiration_hours: i64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct CorsConfig {
    pub allowed_origins: Vec<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct LoggingConfig {
    pub rust_log: String,
    pub log_format: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub name: String,
    pub environment: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Settings {
    pub database: DatabaseConfig,
    pub server: ServerConfig,
    pub jwt: JwtConfig,
    pub cors: CorsConfig,
    pub logging: LoggingConfig,
    pub app: AppConfig,
}

impl Settings {
    /// Load configuration from files and environment variables
    pub fn load() -> AppResult<Self> {
        // Load from default configuration files
        let config = Config::builder()
            .add_source(File::with_name("config/default").required(false))
            .add_source(File::with_name("config/local").required(false))
            .add_source(
                File::with_name(&format!(
                    "config/{}",
                    env::var("APP_ENVIRONMENT").unwrap_or_else(|_| "development".to_string())
                ))
                .required(false),
            )
            .add_source(Environment::with_prefix("APP").separator("__"))
            .build()?;

        // Convert to Settings
        let mut settings: Settings = config.try_deserialize()?;

        // Override with environment variables for sensitive data
        if let Ok(database_url) = env::var("DATABASE_URL") {
            settings.database.url = database_url;
        }
        if let Ok(jwt_secret) = env::var("JWT_SECRET") {
            settings.jwt.secret = jwt_secret;
        }
        if let Ok(rust_log) = env::var("RUST_LOG") {
            settings.logging.rust_log = rust_log;
        }

        // Validate configuration
        settings.validate()?;

        Ok(settings)
    }

    /// Validate configuration values
    fn validate(&self) -> AppResult<()> {
        if self.database.url.is_empty() {
            return Err(AppError::Config("Database URL cannot be empty".to_string()));
        }
        if self.jwt.secret.is_empty() {
            return Err(AppError::Config("JWT secret cannot be empty".to_string()));
        }
        if self.jwt.secret.len() < 32 {
            return Err(AppError::Config(
                "JWT secret must be at least 32 characters".to_string(),
            ));
        }
        if self.database.max_connections == 0 {
            return Err(AppError::Config(
                "Database max connections must be greater than 0".to_string(),
            ));
        }

        // Refuse to boot in production with insecure fallback credentials.
        if self.is_production() {
            if self.jwt.secret == INSECURE_DEFAULT_JWT_SECRET {
                return Err(AppError::Config(
                    "JWT_SECRET must be set to a strong value in production".to_string(),
                ));
            }
            if self.database.url.contains(INSECURE_DEFAULT_DB_PASSWORD) {
                return Err(AppError::Config(
                    "DATABASE_URL must not use the default password in production".to_string(),
                ));
            }
        }

        Ok(())
    }

    /// True when running in the production environment.
    pub fn is_production(&self) -> bool {
        self.app.environment.eq_ignore_ascii_case("production")
    }

    /// Get database URL
    pub fn database_url(&self) -> &str {
        &self.database.url
    }

    /// Get server address
    pub fn server_address(&self) -> String {
        format!("{}:{}", self.server.host, self.server.port)
    }
}

/// Initialize configuration from environment variables (simplified version)
pub fn init_config() -> AppResult<Settings> {
    dotenvy::dotenv().ok();

    let database_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://postgres:password@localhost:5432/pos_system".to_string());

    let jwt_secret =
        env::var("JWT_SECRET").unwrap_or_else(|_| INSECURE_DEFAULT_JWT_SECRET.to_string());

    let rust_log = env::var("RUST_LOG").unwrap_or_else(|_| "debug".to_string());

    let settings = Settings {
        database: DatabaseConfig {
            url: database_url,
            max_connections: env::var("DATABASE_MAX_CONNECTIONS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(10),
            min_connections: env::var("DATABASE_MIN_CONNECTIONS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(5),
        },
        server: ServerConfig {
            host: env::var("SERVER_HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: env::var("SERVER_PORT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(8080),
        },
        jwt: JwtConfig {
            secret: jwt_secret,
            expiration_hours: env::var("JWT_EXPIRATION_HOURS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(24),
        },
        cors: CorsConfig {
            allowed_origins: env::var("CORS_ALLOWED_ORIGINS")
                .unwrap_or_else(|_| "http://localhost:3000,http://localhost:5173".to_string())
                .split(',')
                .map(|s| s.trim().to_string())
                .collect(),
        },
        logging: LoggingConfig {
            rust_log,
            log_format: env::var("LOG_FORMAT").unwrap_or_else(|_| "json".to_string()),
        },
        app: AppConfig {
            name: env::var("APP_NAME").unwrap_or_else(|_| "POS System".to_string()),
            environment: env::var("APP_ENVIRONMENT").unwrap_or_else(|_| "development".to_string()),
        },
    };

    settings.validate()?;
    Ok(settings)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings_with(env: &str, secret: &str, db_url: &str) -> Settings {
        Settings {
            database: DatabaseConfig {
                url: db_url.to_string(),
                max_connections: 10,
                min_connections: 1,
            },
            server: ServerConfig {
                host: "0.0.0.0".to_string(),
                port: 8080,
            },
            jwt: JwtConfig {
                secret: secret.to_string(),
                expiration_hours: 24,
            },
            cors: CorsConfig {
                allowed_origins: vec!["http://localhost:3000".to_string()],
            },
            logging: LoggingConfig {
                rust_log: "info".to_string(),
                log_format: "json".to_string(),
            },
            app: AppConfig {
                name: "POS".to_string(),
                environment: env.to_string(),
            },
        }
    }

    const STRONG_SECRET: &str = "a-genuinely-strong-production-secret-value";
    const PROD_DB: &str = "postgresql://pos:strong-pass@db.internal:5432/pos";

    #[test]
    fn production_rejects_default_jwt_secret() {
        let s = settings_with("production", INSECURE_DEFAULT_JWT_SECRET, PROD_DB);
        assert!(s.validate().is_err());
    }

    #[test]
    fn production_rejects_default_db_password() {
        let s = settings_with(
            "production",
            STRONG_SECRET,
            "postgresql://postgres:password@localhost:5432/pos",
        );
        assert!(s.validate().is_err());
    }

    #[test]
    fn production_accepts_strong_credentials() {
        let s = settings_with("production", STRONG_SECRET, PROD_DB);
        assert!(s.validate().is_ok());
    }

    #[test]
    fn development_allows_insecure_defaults() {
        let s = settings_with(
            "development",
            INSECURE_DEFAULT_JWT_SECRET,
            "postgresql://postgres:password@localhost:5432/pos",
        );
        assert!(s.validate().is_ok());
    }

    #[test]
    fn short_jwt_secret_always_rejected() {
        let s = settings_with("development", "too-short", PROD_DB);
        assert!(s.validate().is_err());
    }

    #[test]
    fn is_production_is_case_insensitive() {
        assert!(settings_with("Production", STRONG_SECRET, PROD_DB).is_production());
        assert!(!settings_with("development", STRONG_SECRET, PROD_DB).is_production());
    }
}
