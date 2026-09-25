use crate::error::AppError;
use crate::restaurant::domain::modifiers::{
    CreateModifierGroupRequest, CreateModifierRequest, LinkProductToModifierGroupRequest,
    UpdateModifierGroupRequest, UpdateModifierRequest,
};
use crate::restaurant::routes::RestaurantAppState;
use crate::restaurant::services::ModifierService;
use axum::{
    extract::{Path, State},
    Json,
};
use validator::Validate;

// Modifier Group Handlers
pub async fn create_modifier_group(
    State(app_state): State<RestaurantAppState>,
    Json(request): Json<CreateModifierGroupRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    request
        .validate()
        .map_err(|e| AppError::BadRequest(e.to_string()))?;

    let modifier_service = ModifierService::new(&app_state.database);
    let group = modifier_service.create_modifier_group(request).await?;

    Ok(Json(serde_json::json!({
        "success": true,
        "data": group
    })))
}

pub async fn get_modifier_group(
    State(app_state): State<RestaurantAppState>,
    Path(id): Path<i32>,
) -> Result<Json<serde_json::Value>, AppError> {
    let modifier_service = ModifierService::new(&app_state.database);
    let group = modifier_service.get_modifier_group(id).await?;

    Ok(Json(serde_json::json!({
        "success": true,
        "data": group
    })))
}

pub async fn list_modifier_groups(
    State(app_state): State<RestaurantAppState>,
    axum::extract::Query(params): axum::extract::Query<ModifierGroupQueryParams>,
) -> Result<Json<serde_json::Value>, AppError> {
    let modifier_service = ModifierService::new(&app_state.database);
    let groups = modifier_service
        .list_modifier_groups(params.is_active)
        .await?;

    Ok(Json(serde_json::json!({
        "success": true,
        "data": groups,
        "count": groups.len()
    })))
}

pub async fn update_modifier_group(
    State(app_state): State<RestaurantAppState>,
    Path(id): Path<i32>,
    Json(request): Json<UpdateModifierGroupRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    request
        .validate()
        .map_err(|e| AppError::BadRequest(e.to_string()))?;

    let modifier_service = ModifierService::new(&app_state.database);
    let group = modifier_service.update_modifier_group(id, request).await?;

    Ok(Json(serde_json::json!({
        "success": true,
        "data": group
    })))
}

pub async fn delete_modifier_group(
    State(app_state): State<RestaurantAppState>,
    Path(id): Path<i32>,
) -> Result<Json<serde_json::Value>, AppError> {
    let modifier_service = ModifierService::new(&app_state.database);
    modifier_service.delete_modifier_group(id).await?;

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Modifier group deleted successfully"
    })))
}

// Modifier Handlers
pub async fn create_modifier(
    State(app_state): State<RestaurantAppState>,
    Json(request): Json<CreateModifierRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    request
        .validate()
        .map_err(|e| AppError::BadRequest(e.to_string()))?;

    let modifier_service = ModifierService::new(&app_state.database);
    let modifier = modifier_service.create_modifier(request).await?;

    Ok(Json(serde_json::json!({
        "success": true,
        "data": modifier
    })))
}

pub async fn get_modifier(
    State(app_state): State<RestaurantAppState>,
    Path(id): Path<i32>,
) -> Result<Json<serde_json::Value>, AppError> {
    let modifier_service = ModifierService::new(&app_state.database);
    let modifier = modifier_service.get_modifier(id).await?;

    Ok(Json(serde_json::json!({
        "success": true,
        "data": modifier
    })))
}

pub async fn update_modifier(
    State(app_state): State<RestaurantAppState>,
    Path(id): Path<i32>,
    Json(request): Json<UpdateModifierRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    request
        .validate()
        .map_err(|e| AppError::BadRequest(e.to_string()))?;

    let modifier_service = ModifierService::new(&app_state.database);
    let modifier = modifier_service.update_modifier(id, request).await?;

    Ok(Json(serde_json::json!({
        "success": true,
        "data": modifier
    })))
}

pub async fn delete_modifier(
    State(app_state): State<RestaurantAppState>,
    Path(id): Path<i32>,
) -> Result<Json<serde_json::Value>, AppError> {
    let modifier_service = ModifierService::new(&app_state.database);
    modifier_service.delete_modifier(id).await?;

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Modifier deleted successfully"
    })))
}

// Product-Modifier Group Linking Handlers
pub async fn link_product_to_modifier_group(
    State(app_state): State<RestaurantAppState>,
    Json(request): Json<LinkProductToModifierGroupRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let modifier_service = ModifierService::new(&app_state.database);
    modifier_service
        .link_product_to_modifier_group(request)
        .await?;

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Product linked to modifier group successfully"
    })))
}

pub async fn unlink_product_from_modifier_group(
    State(app_state): State<RestaurantAppState>,
    Path((product_id, modifier_group_id)): Path<(i32, i32)>,
) -> Result<Json<serde_json::Value>, AppError> {
    let modifier_service = ModifierService::new(&app_state.database);
    modifier_service
        .unlink_product_from_modifier_group(product_id, modifier_group_id)
        .await?;

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Product unlinked from modifier group successfully"
    })))
}

pub async fn get_product_modifier_groups(
    State(app_state): State<RestaurantAppState>,
    Path(product_id): Path<i32>,
) -> Result<Json<serde_json::Value>, AppError> {
    let modifier_service = ModifierService::new(&app_state.database);
    let groups = modifier_service
        .get_product_modifier_groups(product_id)
        .await?;

    Ok(Json(serde_json::json!({
        "success": true,
        "data": groups,
        "count": groups.len()
    })))
}

#[derive(serde::Deserialize)]
pub struct ModifierGroupQueryParams {
    is_active: Option<bool>,
}
