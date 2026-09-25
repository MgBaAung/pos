use crate::error::{AppError, AppResult};
use crate::middleware::auth::AuthContext;
use axum::extract::{FromRequestParts, Request};
use axum::http::request::Parts;

/// Extract branch context from authentication context
pub fn extract_branch_context(request: &Request) -> AppResult<BranchContext> {
    let auth_context = crate::middleware::auth::extract_auth_context(request)?;

    Ok(BranchContext {
        user_id: auth_context.user_id,
        branch_id: auth_context.branch_id,
        can_access_all_branches: auth_context.can_access_all_branches,
    })
}

/// Branch context for filtering operations
#[derive(Debug, Clone)]
pub struct BranchContext {
    pub user_id: i32,
    pub branch_id: Option<i32>,
    pub can_access_all_branches: bool,
}

impl BranchContext {
    /// Get the branch ID for filtering operations
    /// Returns Some(branch_id) if user has specific branch access
    /// Returns None if user can access all branches (for cross-branch operations)
    pub fn get_filter_branch_id(&self) -> Option<i32> {
        if self.can_access_all_branches {
            None // Can access all branches, don't filter
        } else {
            self.branch_id // Filter to user's specific branch
        }
    }

    /// Check if user can access a specific branch
    pub fn can_access_branch(&self, branch_id: i32) -> bool {
        if self.can_access_all_branches {
            return true;
        }
        self.branch_id == Some(branch_id)
    }

    /// Validate that user has access to the specified branch
    pub fn validate_branch_access(&self, branch_id: i32) -> AppResult<()> {
        if !self.can_access_branch(branch_id) {
            return Err(AppError::Auth(
                "You don't have access to this branch".to_string(),
            ));
        }
        Ok(())
    }
}

#[axum::async_trait]
impl<S> FromRequestParts<S> for BranchContext
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let auth_context = AuthContext::from_request_parts(parts, state).await?;

        Ok(BranchContext {
            user_id: auth_context.user_id,
            branch_id: auth_context.branch_id,
            can_access_all_branches: auth_context.can_access_all_branches,
        })
    }
}
