use crate::auth::JwtService;
use crate::database::Database;
use crate::error::AppResult;
use crate::middleware::auth::AuthContext;
use crate::middleware::rbac::Permission;
use crate::models::{
    AssignUserToBranchRequest, BranchComparison, BranchInventorySummary, BranchPerformance,
    BranchResponse, CreateBranchRequest, UpdateBranchRequest, UserBranchAccess,
    UserBranchAssignment,
};
use crate::service::BranchService;
use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;
use std::sync::Arc;
#[derive(Debug, Deserialize)]
pub struct BranchQuery {
    pub include_inactive: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct BranchPerformanceQuery {
    pub branch_id: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct BranchInventoryQuery {
    pub branch_id: Option<i32>,
}

/// Create a new branch (Admin only)
pub async fn create_branch(
    State((_, _, branch_service)): State<(Arc<Database>, Arc<JwtService>, Arc<BranchService>)>,
    auth: AuthContext,
    Json(request): Json<CreateBranchRequest>,
) -> AppResult<Json<BranchResponse>> {
    auth.require_permission(Permission::CreateBranch)?;
    let branch = branch_service.create_branch(request).await?;
    Ok(Json(BranchResponse::from(branch)))
}

/// Get all branches
pub async fn list_branches(
    State((_, _, branch_service)): State<(Arc<Database>, Arc<JwtService>, Arc<BranchService>)>,
    Query(query): Query<BranchQuery>,
) -> AppResult<Json<Vec<BranchResponse>>> {
    let include_inactive = query.include_inactive.unwrap_or(false);
    let branches = branch_service.get_all_branches(include_inactive).await?;
    let responses: Vec<BranchResponse> = branches.into_iter().map(BranchResponse::from).collect();
    Ok(Json(responses))
}

/// Get branch by ID
pub async fn get_branch(
    State((_, _, branch_service)): State<(Arc<Database>, Arc<JwtService>, Arc<BranchService>)>,
    Path(branch_id): Path<i32>,
) -> AppResult<Json<BranchResponse>> {
    let branch = branch_service.get_branch_by_id(branch_id).await?;
    Ok(Json(BranchResponse::from(branch)))
}

/// Get branch by code
pub async fn get_branch_by_code(
    State((_, _, branch_service)): State<(Arc<Database>, Arc<JwtService>, Arc<BranchService>)>,
    Path(code): Path<String>,
) -> AppResult<Json<BranchResponse>> {
    let branch = branch_service.get_branch_by_code(&code).await?;
    Ok(Json(BranchResponse::from(branch)))
}

/// Update branch (Admin only)
pub async fn update_branch(
    State((_, _, branch_service)): State<(Arc<Database>, Arc<JwtService>, Arc<BranchService>)>,
    auth: AuthContext,
    Path(branch_id): Path<i32>,
    Json(request): Json<UpdateBranchRequest>,
) -> AppResult<Json<BranchResponse>> {
    auth.require_permission(Permission::UpdateBranch)?;
    let branch = branch_service.update_branch(branch_id, request).await?;
    Ok(Json(BranchResponse::from(branch)))
}

/// Delete branch (Admin only)
pub async fn delete_branch(
    State((_, _, branch_service)): State<(Arc<Database>, Arc<JwtService>, Arc<BranchService>)>,
    auth: AuthContext,
    Path(branch_id): Path<i32>,
) -> AppResult<Json<serde_json::Value>> {
    auth.require_permission(Permission::DeleteBranch)?;
    branch_service.delete_branch(branch_id).await?;
    Ok(Json(serde_json::json!({
        "message": "Branch deleted successfully"
    })))
}

/// Get branch performance metrics
pub async fn get_branch_performance(
    State((_, _, branch_service)): State<(Arc<Database>, Arc<JwtService>, Arc<BranchService>)>,
    Query(query): Query<BranchPerformanceQuery>,
) -> AppResult<Json<Vec<BranchPerformance>>> {
    let performances = branch_service
        .get_branch_performance(query.branch_id)
        .await?;
    Ok(Json(performances))
}

/// Get branch comparison metrics
pub async fn get_branch_comparison(
    State((_, _, branch_service)): State<(Arc<Database>, Arc<JwtService>, Arc<BranchService>)>,
) -> AppResult<Json<Vec<BranchComparison>>> {
    let comparisons = branch_service.get_branch_comparison().await?;
    Ok(Json(comparisons))
}

/// Get branch inventory summary
pub async fn get_branch_inventory_summary(
    State((_, _, branch_service)): State<(Arc<Database>, Arc<JwtService>, Arc<BranchService>)>,
    Query(query): Query<BranchInventoryQuery>,
) -> AppResult<Json<Vec<BranchInventorySummary>>> {
    let summaries = branch_service
        .get_branch_inventory_summary(query.branch_id)
        .await?;
    Ok(Json(summaries))
}

/// Assign user to branch (Admin only)
pub async fn assign_user_to_branch(
    State((_, _, branch_service)): State<(Arc<Database>, Arc<JwtService>, Arc<BranchService>)>,
    auth: AuthContext,
    Json(request): Json<AssignUserToBranchRequest>,
) -> AppResult<Json<UserBranchAssignment>> {
    auth.require_permission(Permission::AssignUserToBranch)?;
    let assignment = branch_service
        .assign_user_to_branch(request, auth.user_id)
        .await?;
    Ok(Json(assignment))
}

/// Remove user from branch (Admin only)
pub async fn remove_user_from_branch(
    State((_, _, branch_service)): State<(Arc<Database>, Arc<JwtService>, Arc<BranchService>)>,
    auth: AuthContext,
    Path((user_id, branch_id)): Path<(i32, i32)>,
) -> AppResult<Json<serde_json::Value>> {
    auth.require_permission(Permission::RemoveUserFromBranch)?;
    branch_service
        .remove_user_from_branch(user_id, branch_id)
        .await?;
    Ok(Json(serde_json::json!({
        "message": "User removed from branch successfully"
    })))
}

/// Get user branch access
pub async fn get_user_branch_access(
    State((_, _, branch_service)): State<(Arc<Database>, Arc<JwtService>, Arc<BranchService>)>,
    Path(user_id): Path<i32>,
) -> AppResult<Json<UserBranchAccess>> {
    let access = branch_service.get_user_branch_access(user_id).await?;
    Ok(Json(access))
}

/// Get user branch assignments
pub async fn get_user_branch_assignments(
    State((_, _, branch_service)): State<(Arc<Database>, Arc<JwtService>, Arc<BranchService>)>,
    Path(user_id): Path<i32>,
) -> AppResult<Json<Vec<UserBranchAssignment>>> {
    let assignments = branch_service.get_user_branch_assignments(user_id).await?;
    Ok(Json(assignments))
}

/// Get branch users
pub async fn get_branch_users(
    State((_, _, branch_service)): State<(Arc<Database>, Arc<JwtService>, Arc<BranchService>)>,
    Path(branch_id): Path<i32>,
) -> AppResult<Json<Vec<crate::models::User>>> {
    let users = branch_service.get_branch_users(branch_id).await?;
    Ok(Json(users))
}
