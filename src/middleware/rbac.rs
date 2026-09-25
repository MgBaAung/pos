use crate::error::AppError;
use crate::middleware::extract_auth_context;
use crate::models::UserRole;
use axum::{
    extract::Request,
    middleware::Next,
    response::{IntoResponse, Response},
};

/// Role-based access control check
pub async fn require_role(allowed_roles: Vec<UserRole>, request: Request, next: Next) -> Response {
    // Extract auth context
    let auth_context = match extract_auth_context(&request) {
        Ok(context) => context,
        Err(e) => {
            tracing::warn!("RBAC check failed: {}", e);
            return e.into_response();
        }
    };

    // Check if user has required role
    if !allowed_roles.contains(&auth_context.role) {
        tracing::warn!(
            "User {} with role {:?} attempted to access restricted resource",
            auth_context.username,
            auth_context.role
        );
        return AppError::Authorization("Insufficient permissions for this operation".to_string())
            .into_response();
    }

    // Continue to next handler
    next.run(request).await
}

/// Convenience function to require admin role
pub async fn require_admin(request: Request, next: Next) -> Response {
    require_role(vec![UserRole::Admin], request, next).await
}

/// Convenience function to require admin or store manager role
pub async fn require_manager_or_admin(request: Request, next: Next) -> Response {
    require_role(vec![UserRole::Admin, UserRole::StoreManager], request, next).await
}

/// Convenience function to require cashier or higher role
pub async fn require_cashier_or_higher(request: Request, next: Next) -> Response {
    require_role(
        vec![UserRole::Admin, UserRole::StoreManager, UserRole::Cashier],
        request,
        next,
    )
    .await
}

/// Role hierarchy for permission checks
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Role {
    Cashier = 1,
    StoreManager = 2,
    Admin = 3,
}

impl From<UserRole> for Role {
    fn from(user_role: UserRole) -> Self {
        match user_role {
            UserRole::Cashier => Role::Cashier,
            UserRole::StoreManager => Role::StoreManager,
            UserRole::Admin => Role::Admin,
        }
    }
}

impl From<Role> for UserRole {
    fn from(role: Role) -> Self {
        match role {
            Role::Cashier => UserRole::Cashier,
            Role::StoreManager => UserRole::StoreManager,
            Role::Admin => UserRole::Admin,
        }
    }
}

/// Check if a user role has sufficient permissions
pub fn has_permission(user_role: UserRole, required_role: Role) -> bool {
    Role::from(user_role) >= required_role
}

/// Permission definitions for different operations
#[derive(Debug, Clone, Copy)]
pub enum Permission {
    // User management
    CreateUser,
    UpdateUser,
    DeleteUser,
    ViewUsers,

    // Product management
    CreateProduct,
    UpdateProduct,
    DeleteProduct,
    ViewProducts,

    // Category management
    CreateCategory,
    UpdateCategory,
    DeleteCategory,
    ViewCategories,

    // Inventory management
    AdjustStock,
    ViewInventoryLogs,

    // Sales operations
    CreateSale,
    ViewSales,
    RefundSale,

    // Customer management
    CreateCustomer,
    UpdateCustomer,
    ViewCustomers,

    // Reports
    ViewReports,
    ViewFinancialReports,

    // Cash drawer
    OpenCashDrawer,
    CloseCashDrawer,

    // Branch management - Admin only
    CreateBranch,
    UpdateBranch,
    DeleteBranch,
    AssignUserToBranch,
    RemoveUserFromBranch,
}

impl Permission {
    /// Get the minimum role required for this permission
    pub fn required_role(&self) -> Role {
        match self {
            // User management - Admin only
            Permission::CreateUser => Role::Admin,
            Permission::UpdateUser => Role::Admin,
            Permission::DeleteUser => Role::Admin,
            Permission::ViewUsers => Role::Admin,

            // Product management - Manager or Admin
            Permission::CreateProduct => Role::StoreManager,
            Permission::UpdateProduct => Role::StoreManager,
            Permission::DeleteProduct => Role::Admin,
            Permission::ViewProducts => Role::Cashier,

            // Category management - Manager or Admin
            Permission::CreateCategory => Role::StoreManager,
            Permission::UpdateCategory => Role::StoreManager,
            Permission::DeleteCategory => Role::Admin,
            Permission::ViewCategories => Role::Cashier,

            // Inventory management - Manager or Admin
            Permission::AdjustStock => Role::StoreManager,
            Permission::ViewInventoryLogs => Role::StoreManager,

            // Sales operations - Cashier and above
            Permission::CreateSale => Role::Cashier,
            Permission::ViewSales => Role::Cashier,
            Permission::RefundSale => Role::StoreManager,

            // Customer management - Cashier and above
            Permission::CreateCustomer => Role::Cashier,
            Permission::UpdateCustomer => Role::Cashier,
            Permission::ViewCustomers => Role::Cashier,

            // Reports - Manager and above
            Permission::ViewReports => Role::StoreManager,
            Permission::ViewFinancialReports => Role::Admin,

            // Cash drawer - Cashier and above
            Permission::OpenCashDrawer => Role::Cashier,
            Permission::CloseCashDrawer => Role::Cashier,

            // Branch management - Admin only
            Permission::CreateBranch => Role::Admin,
            Permission::UpdateBranch => Role::Admin,
            Permission::DeleteBranch => Role::Admin,
            Permission::AssignUserToBranch => Role::Admin,
            Permission::RemoveUserFromBranch => Role::Admin,
        }
    }

    /// Check if a user role has this permission
    pub fn check(&self, user_role: UserRole) -> bool {
        has_permission(user_role, self.required_role())
    }
}
