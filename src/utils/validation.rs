use crate::error::{AppError, AppResult};
use validator::Validate;

/// Validate a request using the validator crate
pub fn validate_request<T: Validate>(request: &T) -> AppResult<()> {
    request.validate().map_err(|errors| {
        let error_messages: Vec<String> = errors
            .field_errors()
            .iter()
            .flat_map(|(field, errors)| {
                errors.iter().map(move |error| {
                    format!(
                        "{}: {}",
                        field,
                        error
                            .message
                            .as_ref()
                            .map(|m| m.to_string())
                            .unwrap_or_else(|| "validation error".to_string())
                    )
                })
            })
            .collect();

        AppError::Validation(error_messages.join(", "))
    })
}
