use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

/// Custom error types for the POS system
#[derive(Error, Debug)]
pub enum AppError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Authentication error: {0}")]
    Auth(String),

    #[error("Authorization error: {0}")]
    Authorization(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Insufficient stock: {0}")]
    InsufficientStock(String),

    #[error("Payment error: {0}")]
    Payment(String),

    #[error("Internal server error: {0}")]
    Internal(String),

    #[error("JWT error: {0}")]
    Jwt(#[from] jsonwebtoken::errors::Error),

    #[error("Password hashing error: {0}")]
    PasswordHash(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Configuration load error: {0}")]
    ConfigLoad(#[from] config::ConfigError),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            AppError::Database(ref e) => {
                tracing::error!("Database error: {}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, "Database error occurred")
            }
            AppError::Auth(ref msg) => {
                tracing::warn!("Authentication error: {}", msg);
                (StatusCode::UNAUTHORIZED, msg.as_str())
            }
            AppError::Authorization(ref msg) => {
                tracing::warn!("Authorization error: {}", msg);
                (StatusCode::FORBIDDEN, msg.as_str())
            }
            AppError::Validation(ref msg) => {
                tracing::warn!("Validation error: {}", msg);
                (StatusCode::BAD_REQUEST, msg.as_str())
            }
            AppError::BadRequest(ref msg) => {
                tracing::warn!("Bad request: {}", msg);
                (StatusCode::BAD_REQUEST, msg.as_str())
            }
            AppError::NotFound(ref msg) => {
                tracing::debug!("Not found: {}", msg);
                (StatusCode::NOT_FOUND, msg.as_str())
            }
            AppError::Conflict(ref msg) => {
                tracing::warn!("Conflict: {}", msg);
                (StatusCode::CONFLICT, msg.as_str())
            }
            AppError::InsufficientStock(ref msg) => {
                tracing::warn!("Insufficient stock: {}", msg);
                (StatusCode::BAD_REQUEST, msg.as_str())
            }
            AppError::Payment(ref msg) => {
                tracing::warn!("Payment error: {}", msg);
                (StatusCode::BAD_REQUEST, msg.as_str())
            }
            AppError::Internal(ref msg) => {
                tracing::error!("Internal error: {}", msg);
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error")
            }
            AppError::Jwt(ref e) => {
                tracing::warn!("JWT error: {}", e);
                (StatusCode::UNAUTHORIZED, "Invalid or expired token")
            }
            AppError::PasswordHash(ref msg) => {
                tracing::error!("Password hash error: {}", msg);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Password processing error",
                )
            }
            AppError::Config(ref msg) => {
                tracing::error!("Configuration error: {}", msg);
                (StatusCode::INTERNAL_SERVER_ERROR, "Configuration error")
            }
            AppError::ConfigLoad(ref e) => {
                tracing::error!("Configuration load error: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Configuration load error",
                )
            }
        };

        let body = json!({
            "error": error_message,
            "status": status.as_u16(),
        });

        (status, Json(body)).into_response()
    }
}

/// Result type alias for AppError
pub type AppResult<T> = Result<T, AppError>;
