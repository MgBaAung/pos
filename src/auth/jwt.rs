use crate::error::{AppError, AppResult};
use crate::models::UserRole;
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String, // user_id
    pub username: String,
    pub role: UserRole,
    pub branch_id: Option<i32>,
    pub can_access_all_branches: bool,
    pub exp: i64, // expiration time
    pub iat: i64, // issued at
}

pub struct JwtService {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    expiration_hours: i64,
}

impl JwtService {
    pub fn new(secret: &str, expiration_hours: i64) -> Self {
        JwtService {
            encoding_key: EncodingKey::from_secret(secret.as_bytes()),
            decoding_key: DecodingKey::from_secret(secret.as_bytes()),
            expiration_hours,
        }
    }

    /// Generate a JWT token for a user
    pub fn generate_token(
        &self,
        user_id: i32,
        username: &str,
        role: UserRole,
        branch_id: Option<i32>,
        can_access_all_branches: bool,
    ) -> AppResult<String> {
        let now = Utc::now();
        let exp = now + Duration::hours(self.expiration_hours);

        let claims = Claims {
            sub: user_id.to_string(),
            username: username.to_string(),
            role,
            branch_id,
            can_access_all_branches,
            exp: exp.timestamp(),
            iat: now.timestamp(),
        };

        encode(&Header::default(), &claims, &self.encoding_key).map_err(|e| {
            tracing::error!("Failed to generate JWT token: {}", e);
            AppError::Jwt(e)
        })
    }

    /// Validate and decode a JWT token
    pub fn validate_token(&self, token: &str) -> AppResult<Claims> {
        let validation = Validation::new(Algorithm::HS256);

        decode::<Claims>(token, &self.decoding_key, &validation)
            .map(|data| data.claims)
            .map_err(|e| {
                tracing::warn!("Failed to validate JWT token: {}", e);
                AppError::Jwt(e)
            })
    }

    /// Extract user_id from token
    pub fn extract_user_id(&self, token: &str) -> AppResult<i32> {
        let claims = self.validate_token(token)?;
        claims
            .sub
            .parse::<i32>()
            .map_err(|e| AppError::Auth(format!("Invalid user ID in token: {}", e)))
    }

    /// Extract username from token
    pub fn extract_username(&self, token: &str) -> AppResult<String> {
        let claims = self.validate_token(token)?;
        Ok(claims.username)
    }

    /// Extract role from token
    pub fn extract_role(&self, token: &str) -> AppResult<UserRole> {
        let claims = self.validate_token(token)?;
        Ok(claims.role)
    }
}
