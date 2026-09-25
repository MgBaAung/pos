use crate::auth::{JwtService, PasswordService};
use crate::database::Database;
use crate::error::{AppError, AppResult};
use crate::middleware::auth::{extract_auth_context, AuthContext};
use crate::models::{AuthResponse, CreateUserRequest, LoginRequest, User, UserResponse};
use axum::{
    extract::{Request, State},
    Json,
};
use serde::Deserialize;
use std::sync::Arc;

#[derive(Debug, Deserialize)]
pub struct SwitchBranchRequest {
    pub branch_id: i32,
}

pub async fn register(
    State((db, jwt_service)): State<(Arc<Database>, Arc<JwtService>)>,
    Json(request): Json<CreateUserRequest>,
) -> AppResult<Json<AuthResponse>> {
    // Validate password strength
    PasswordService::validate_password_strength(&request.password)?;

    // Hash password
    let password_hash = PasswordService::hash_password(&request.password)?;

    // Check if username already exists
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

    // Validate branch if provided
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

    // Create user
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

    // Generate JWT token
    let token = jwt_service.generate_token(
        user.id,
        &user.username,
        user.role,
        user.branch_id,
        user.can_access_all_branches,
    )?;

    tracing::info!("User registered: {}", user.username);

    Ok(Json(AuthResponse {
        user: UserResponse::from(user),
        token,
    }))
}

pub async fn login(
    State((db, jwt_service)): State<(Arc<Database>, Arc<JwtService>)>,
    Json(request): Json<LoginRequest>,
) -> AppResult<Json<AuthResponse>> {
    // Get user by username
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE username = $1")
        .bind(&request.username)
        .fetch_optional(&db.pool)
        .await?
        .ok_or_else(|| AppError::Auth("Invalid username or password".to_string()))?;

    // Check if user is active
    if !user.is_active {
        return Err(AppError::Auth("User account is disabled".to_string()));
    }

    // Verify password
    PasswordService::verify_password(&request.password, &user.password_hash)?;

    // Generate JWT token
    let token = jwt_service.generate_token(
        user.id,
        &user.username,
        user.role,
        user.branch_id,
        user.can_access_all_branches,
    )?;

    tracing::info!("User logged in: {}", user.username);

    Ok(Json(AuthResponse {
        user: UserResponse::from(user),
        token,
    }))
}

pub async fn me(
    State((db, _jwt_service)): State<(Arc<Database>, Arc<JwtService>)>,
    request: Request,
) -> AppResult<Json<UserResponse>> {
    let auth_context = extract_auth_context(&request)?;

    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
        .bind(auth_context.user_id)
        .fetch_one(&db.pool)
        .await?;

    Ok(Json(UserResponse::from(user)))
}

pub async fn switch_branch(
    State((db, jwt_service)): State<(Arc<Database>, Arc<JwtService>)>,
    auth_context: AuthContext,
    Json(switch_request): Json<SwitchBranchRequest>,
) -> AppResult<Json<AuthResponse>> {
    // Check if user has access to the requested branch
    let has_access = if auth_context.can_access_all_branches {
        true
    } else {
        // Check if user is assigned to this branch
        let assignment = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM user_branch_assignments WHERE user_id = $1 AND branch_id = $2)"
        )
        .bind(auth_context.user_id)
        .bind(switch_request.branch_id)
        .fetch_one(&db.pool)
        .await?;

        assignment
    };

    if !has_access {
        return Err(AppError::Auth(
            "You don't have access to this branch".to_string(),
        ));
    }

    // Check if branch exists and is active
    let branch = sqlx::query_scalar::<_, bool>("SELECT is_active FROM branches WHERE id = $1")
        .bind(switch_request.branch_id)
        .fetch_optional(&db.pool)
        .await?
        .ok_or_else(|| {
            AppError::NotFound(format!("Branch {} not found", switch_request.branch_id))
        })?;

    if !branch {
        return Err(AppError::Auth("Branch is not active".to_string()));
    }

    // Get user details
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
        .bind(auth_context.user_id)
        .fetch_one(&db.pool)
        .await?;

    // Generate new token with updated branch context
    let token = jwt_service.generate_token(
        user.id,
        &user.username,
        user.role,
        Some(switch_request.branch_id),
        user.can_access_all_branches,
    )?;

    tracing::info!(
        "User {} switched to branch {}",
        user.username,
        switch_request.branch_id
    );

    Ok(Json(AuthResponse {
        user: UserResponse::from(user),
        token,
    }))
}
