use crate::auth::JwtService;
use crate::error::{AppError, AppResult};
use crate::middleware::rbac::Permission;
use crate::models::UserRole;
use axum::{
    extract::{FromRequestParts, Request, State},
    http::{request::Parts, HeaderMap},
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::sync::Arc;

/// Context containing authenticated user information
#[derive(Clone)]
pub struct AuthContext {
    pub user_id: i32,
    pub username: String,
    pub role: UserRole,
    pub branch_id: Option<i32>,
    pub can_access_all_branches: bool,
}

impl AuthContext {
    /// Reject the request unless the user's role satisfies the permission.
    ///
    /// Authorization policy lives in [`Permission::required_role`]; this only
    /// applies it to the authenticated user.
    pub fn require_permission(&self, permission: Permission) -> AppResult<()> {
        if permission.check(self.role) {
            Ok(())
        } else {
            Err(AppError::Authorization(format!(
                "Role '{}' is not permitted to perform this action",
                self.role.as_str()
            )))
        }
    }
}

/// Extract Bearer token from Authorization header
pub fn extract_token(headers: &HeaderMap) -> AppResult<String> {
    let auth_header = headers
        .get("authorization")
        .ok_or_else(|| AppError::Auth("Missing authorization header".to_string()))?
        .to_str()
        .map_err(|_| AppError::Auth("Invalid authorization header".to_string()))?;

    if !auth_header.starts_with("Bearer ") {
        return Err(AppError::Auth(
            "Invalid authorization header format".to_string(),
        ));
    }

    Ok(auth_header[7..].to_string())
}

/// Authentication middleware to validate JWT tokens
pub async fn auth_middleware(
    State(jwt_service): State<Arc<JwtService>>,
    mut request: Request,
    next: Next,
) -> Response {
    // Extract token from Authorization header
    let token = match extract_token(request.headers()) {
        Ok(token) => token,
        Err(e) => {
            tracing::warn!("Authentication failed: {}", e);
            return e.into_response();
        }
    };

    // Validate token and extract claims
    let claims = match jwt_service.validate_token(&token) {
        Ok(claims) => claims,
        Err(e) => {
            tracing::warn!("Token validation failed: {}", e);
            return e.into_response();
        }
    };

    // Parse user_id
    let user_id = match claims.sub.parse::<i32>() {
        Ok(id) => id,
        Err(e) => {
            tracing::error!("Failed to parse user_id from token: {}", e);
            return AppError::Auth("Invalid user ID in token".to_string()).into_response();
        }
    };

    // Add auth context to request extensions
    let auth_context = AuthContext {
        user_id,
        username: claims.username,
        role: claims.role,
        branch_id: claims.branch_id,
        can_access_all_branches: claims.can_access_all_branches,
    };

    request.extensions_mut().insert(auth_context);

    // Continue to next handler
    next.run(request).await
}

/// Extractor for AuthContext from request extensions
pub fn extract_auth_context(request: &Request) -> AppResult<AuthContext> {
    request
        .extensions()
        .get::<AuthContext>()
        .cloned()
        .ok_or_else(|| AppError::Auth("No auth context found".to_string()))
}

#[axum::async_trait]
impl<S> FromRequestParts<S> for AuthContext
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<AuthContext>()
            .cloned()
            .ok_or_else(|| AppError::Auth("No auth context found".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(role: UserRole) -> AuthContext {
        AuthContext {
            user_id: 7,
            username: "tester".to_string(),
            role,
            branch_id: Some(1),
            can_access_all_branches: false,
        }
    }

    #[test]
    fn cashier_can_create_sale_but_not_products() {
        let cashier = ctx(UserRole::Cashier);
        assert!(cashier.require_permission(Permission::CreateSale).is_ok());
        assert!(cashier
            .require_permission(Permission::CreateProduct)
            .is_err());
    }

    #[test]
    fn manager_can_manage_products_but_not_branches() {
        let manager = ctx(UserRole::StoreManager);
        assert!(manager
            .require_permission(Permission::CreateProduct)
            .is_ok());
        assert!(manager.require_permission(Permission::RefundSale).is_ok());
        assert!(manager
            .require_permission(Permission::CreateBranch)
            .is_err());
    }

    #[test]
    fn admin_can_manage_branches() {
        let admin = ctx(UserRole::Admin);
        assert!(admin.require_permission(Permission::CreateBranch).is_ok());
        assert!(admin
            .require_permission(Permission::RemoveUserFromBranch)
            .is_ok());
    }

    #[test]
    fn cashier_cannot_refund_sale() {
        assert!(ctx(UserRole::Cashier)
            .require_permission(Permission::RefundSale)
            .is_err());
    }
}
