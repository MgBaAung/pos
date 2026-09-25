use crate::auth::PasswordService;
use crate::database::Database;
use crate::error::{AppError, AppResult};
use crate::middleware::auth::AuthContext;
use crate::middleware::rbac::Permission;
use crate::models::{CreateUserRequest, UpdateUserRequest, User, UserResponse};
use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
pub struct ListUsersQuery {
    pub role: Option<String>,
    pub branch_id: Option<i32>,
    pub active_only: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct ResetPasswordRequest {
    pub new_password: String,
}

/// List all users (Admin only).
pub async fn list_users(
    State(db): State<Database>,
    auth: AuthContext,
    Query(params): Query<ListUsersQuery>,
) -> AppResult<Json<Vec<UserResponse>>> {
    auth.require_permission(Permission::ViewUsers)?;

    let mut query = String::from("SELECT * FROM users WHERE 1=1");
    let mut conditions = Vec::new();
    let mut param_count = 0;

    if params.role.is_some() {
        param_count += 1;
        conditions.push(format!("AND role::text = ${}", param_count));
    }
    if params.branch_id.is_some() {
        param_count += 1;
        conditions.push(format!("AND branch_id = ${}", param_count));
    }
    if let Some(true) = params.active_only {
        conditions.push("AND is_active = TRUE".to_string());
    }

    query.push_str(&conditions.join(" "));
    query.push_str(" ORDER BY created_at DESC");

    let mut query_builder = sqlx::query_as::<_, User>(&query);

    if let Some(role) = params.role {
        query_builder = query_builder.bind(role);
    }
    if let Some(branch_id) = params.branch_id {
        query_builder = query_builder.bind(branch_id);
    }

    let users = query_builder.fetch_all(&db.pool).await?;

    let responses: Vec<UserResponse> = users.into_iter().map(UserResponse::from).collect();
    Ok(Json(responses))
}

/// Get a single user (Admin only).
pub async fn get_user(
    State(db): State<Database>,
    auth: AuthContext,
    Path(id): Path<i32>,
) -> AppResult<Json<UserResponse>> {
    auth.require_permission(Permission::ViewUsers)?;

    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
        .bind(id)
        .fetch_optional(&db.pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("User with id {} not found", id)))?;

    Ok(Json(UserResponse::from(user)))
}

/// Create a new user (Admin only).
pub async fn create_user(
    State(db): State<Database>,
    auth: AuthContext,
    Json(request): Json<CreateUserRequest>,
) -> AppResult<Json<UserResponse>> {
    auth.require_permission(Permission::CreateUser)?;

    PasswordService::validate_password_strength(&request.password)?;

    let existing = sqlx::query_scalar::<_, i32>("SELECT id FROM users WHERE username = $1")
        .bind(&request.username)
        .fetch_optional(&db.pool)
        .await?;

    if existing.is_some() {
        return Err(AppError::Conflict(format!(
            "Username {} already exists",
            request.username
        )));
    }

    if let Some(branch_id) = request.branch_id {
        let branch_exists = sqlx::query_scalar::<_, i32>("SELECT id FROM branches WHERE id = $1")
            .bind(branch_id)
            .fetch_optional(&db.pool)
            .await?;

        if branch_exists.is_none() {
            return Err(AppError::NotFound(format!(
                "Branch {} not found",
                branch_id
            )));
        }
    }

    let password_hash = PasswordService::hash_password(&request.password)?;

    let user = sqlx::query_as::<_, User>(
        r#"
        INSERT INTO users (username, password_hash, full_name, role, branch_id, can_access_all_branches)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING *
        "#,
    )
    .bind(&request.username)
    .bind(&password_hash)
    .bind(&request.full_name)
    .bind(request.role)
    .bind(request.branch_id)
    .bind(request.can_access_all_branches.unwrap_or(false))
    .fetch_one(&db.pool)
    .await?;

    tracing::info!("User created by admin {}: {}", auth.user_id, user.username);

    Ok(Json(UserResponse::from(user)))
}

/// Update a user's profile fields (Admin only).
pub async fn update_user(
    State(db): State<Database>,
    auth: AuthContext,
    Path(id): Path<i32>,
    Json(request): Json<UpdateUserRequest>,
) -> AppResult<Json<UserResponse>> {
    auth.require_permission(Permission::UpdateUser)?;

    let mut updates = Vec::new();
    let mut param_count = 0;

    if request.full_name.is_some() {
        param_count += 1;
        updates.push(format!("full_name = ${}", param_count));
    }
    if request.role.is_some() {
        param_count += 1;
        updates.push(format!("role = ${}", param_count));
    }
    if request.is_active.is_some() {
        param_count += 1;
        updates.push(format!("is_active = ${}", param_count));
    }

    if updates.is_empty() {
        return Err(AppError::BadRequest("No fields to update".to_string()));
    }

    updates.push("updated_at = CURRENT_TIMESTAMP".to_string());

    let query = format!(
        "UPDATE users SET {} WHERE id = ${} RETURNING *",
        updates.join(", "),
        param_count + 1
    );

    let mut query_builder = sqlx::query_as::<_, User>(&query);

    if let Some(full_name) = request.full_name {
        query_builder = query_builder.bind(full_name);
    }
    if let Some(role) = request.role {
        query_builder = query_builder.bind(role);
    }
    if let Some(is_active) = request.is_active {
        query_builder = query_builder.bind(is_active);
    }
    query_builder = query_builder.bind(id);

    let user = query_builder
        .fetch_optional(&db.pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("User with id {} not found", id)))?;

    tracing::info!("User {} updated by admin {}", user.id, auth.user_id);

    Ok(Json(UserResponse::from(user)))
}

/// Soft-delete a user by deactivating their account (Admin only).
pub async fn deactivate_user(
    State(db): State<Database>,
    auth: AuthContext,
    Path(id): Path<i32>,
) -> AppResult<Json<HashMap<String, String>>> {
    auth.require_permission(Permission::DeleteUser)?;

    if id == auth.user_id {
        return Err(AppError::BadRequest(
            "You cannot deactivate your own account".to_string(),
        ));
    }

    let user = sqlx::query_as::<_, User>(
        "UPDATE users SET is_active = FALSE, updated_at = CURRENT_TIMESTAMP WHERE id = $1 RETURNING *",
    )
    .bind(id)
    .fetch_optional(&db.pool)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("User with id {} not found", id)))?;

    tracing::info!("User {} deactivated by admin {}", user.id, auth.user_id);

    let mut response = HashMap::new();
    response.insert(
        "message".to_string(),
        "User deactivated successfully".to_string(),
    );
    Ok(Json(response))
}

/// Reset a user's password (Admin only).
pub async fn reset_user_password(
    State(db): State<Database>,
    auth: AuthContext,
    Path(id): Path<i32>,
    Json(request): Json<ResetPasswordRequest>,
) -> AppResult<Json<HashMap<String, String>>> {
    auth.require_permission(Permission::UpdateUser)?;

    PasswordService::validate_password_strength(&request.new_password)?;

    let password_hash = PasswordService::hash_password(&request.new_password)?;

    let user = sqlx::query_as::<_, User>(
        "UPDATE users SET password_hash = $1, updated_at = CURRENT_TIMESTAMP WHERE id = $2 RETURNING *",
    )
    .bind(&password_hash)
    .bind(id)
    .fetch_optional(&db.pool)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("User with id {} not found", id)))?;

    tracing::info!(
        "Password reset for user {} by admin {}",
        user.id,
        auth.user_id
    );

    let mut response = HashMap::new();
    response.insert(
        "message".to_string(),
        "Password reset successfully".to_string(),
    );
    Ok(Json(response))
}
