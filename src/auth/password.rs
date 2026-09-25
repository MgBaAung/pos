use crate::error::{AppError, AppResult};
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Algorithm, Argon2, Params, Version,
};

pub struct PasswordService;

impl PasswordService {
    /// Hash a password using Argon2
    pub fn hash_password(password: &str) -> AppResult<String> {
        let salt = SaltString::generate(&mut OsRng);

        let argon2 = Argon2::new(
            Algorithm::Argon2id,
            Version::V0x13,
            Params::new(65536, 2, 1, None).map_err(|e| {
                AppError::PasswordHash(format!("Failed to create Argon2 params: {}", e))
            })?,
        );

        let password_hash = argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| {
                tracing::error!("Failed to hash password: {}", e);
                AppError::PasswordHash(format!("Failed to hash password: {}", e))
            })?
            .to_string();

        Ok(password_hash)
    }

    /// Verify a password against a hash
    pub fn verify_password(password: &str, hash: &str) -> AppResult<bool> {
        let parsed_hash = PasswordHash::new(hash).map_err(|e| {
            tracing::error!("Failed to parse password hash: {}", e);
            AppError::PasswordHash(format!("Invalid password hash format: {}", e))
        })?;

        let argon2 = Argon2::default();

        argon2
            .verify_password(password.as_bytes(), &parsed_hash)
            .map(|_| true)
            .map_err(|e| {
                tracing::warn!("Password verification failed: {}", e);
                AppError::Auth("Invalid password".to_string())
            })
    }

    /// Validate password strength
    pub fn validate_password_strength(password: &str) -> AppResult<()> {
        if password.len() < 8 {
            return Err(AppError::Validation(
                "Password must be at least 8 characters long".to_string(),
            ));
        }

        if password.len() > 128 {
            return Err(AppError::Validation(
                "Password must not exceed 128 characters".to_string(),
            ));
        }

        // Check for at least one uppercase letter
        if !password.chars().any(|c| c.is_uppercase()) {
            return Err(AppError::Validation(
                "Password must contain at least one uppercase letter".to_string(),
            ));
        }

        // Check for at least one lowercase letter
        if !password.chars().any(|c| c.is_lowercase()) {
            return Err(AppError::Validation(
                "Password must contain at least one lowercase letter".to_string(),
            ));
        }

        // Check for at least one digit
        if !password.chars().any(|c| c.is_ascii_digit()) {
            return Err(AppError::Validation(
                "Password must contain at least one digit".to_string(),
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_password_success() {
        let password = "TestPassword123";
        let hash = PasswordService::hash_password(password).unwrap();

        // Hash should be different from original password
        assert_ne!(hash, password);

        // Hash should contain argon2 identifier
        assert!(hash.contains("$argon2id$"));
    }

    #[test]
    fn test_verify_password_success() {
        let password = "TestPassword123";
        let hash = PasswordService::hash_password(password).unwrap();

        let result = PasswordService::verify_password(password, &hash).unwrap();
        assert!(result);
    }

    #[test]
    fn test_verify_password_failure() {
        let password = "TestPassword123";
        let wrong_password = "WrongPassword123";
        let hash = PasswordService::hash_password(password).unwrap();

        let result = PasswordService::verify_password(wrong_password, &hash);
        assert!(result.is_err());
    }

    #[test]
    fn test_verify_password_invalid_hash() {
        let password = "TestPassword123";
        let invalid_hash = "invalid_hash_format";

        let result = PasswordService::verify_password(password, invalid_hash);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_password_strength_too_short() {
        let password = "Short1";
        let result = PasswordService::validate_password_strength(password);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("at least 8 characters"));
    }

    #[test]
    fn test_validate_password_strength_too_long() {
        let password = "a".repeat(129);
        let result = PasswordService::validate_password_strength(&password);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("not exceed 128 characters"));
    }

    #[test]
    fn test_validate_password_strength_no_uppercase() {
        let password = "lowercase123";
        let result = PasswordService::validate_password_strength(password);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("uppercase letter"));
    }

    #[test]
    fn test_validate_password_strength_no_lowercase() {
        let password = "UPPERCASE123";
        let result = PasswordService::validate_password_strength(password);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("lowercase letter"));
    }

    #[test]
    fn test_validate_password_strength_no_digit() {
        let password = "NoDigitsHere";
        let result = PasswordService::validate_password_strength(password);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("digit"));
    }

    #[test]
    fn test_validate_password_strength_valid() {
        let password = "ValidPassword123";
        let result = PasswordService::validate_password_strength(password);
        assert!(result.is_ok());
    }

    #[test]
    fn test_hash_and_verify_consistency() {
        let password = "ConsistentPassword123";

        // Hash the same password twice should produce different hashes (due to salt)
        let hash1 = PasswordService::hash_password(password).unwrap();
        let hash2 = PasswordService::hash_password(password).unwrap();
        assert_ne!(hash1, hash2);

        // But both should verify successfully
        assert!(PasswordService::verify_password(password, &hash1).unwrap());
        assert!(PasswordService::verify_password(password, &hash2).unwrap());
    }
}
