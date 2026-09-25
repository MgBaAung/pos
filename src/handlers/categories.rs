use crate::database::Database;
use crate::error::AppResult;
use crate::inventory::InventoryService;
use crate::middleware::auth::AuthContext;
use crate::middleware::rbac::Permission;
use crate::models::{Category, CreateCategoryRequest, UpdateCategoryRequest};
use crate::utils::validate_request;
use axum::{
    extract::{Path, State},
    Json,
};
use std::collections::HashMap;

pub async fn list_categories(State(db): State<Database>) -> AppResult<Json<Vec<Category>>> {
    let inventory_service = InventoryService::new(db);
    let categories = inventory_service.list_categories().await?;
    Ok(Json(categories))
}

pub async fn get_category(
    State(db): State<Database>,
    Path(id): Path<i32>,
) -> AppResult<Json<Category>> {
    let inventory_service = InventoryService::new(db);
    let category = inventory_service.get_category(id).await?;
    Ok(Json(category))
}

pub async fn create_category(
    State(db): State<Database>,
    auth: AuthContext,
    Json(category_request): Json<CreateCategoryRequest>,
) -> AppResult<Json<Category>> {
    auth.require_permission(Permission::CreateCategory)?;

    // Validate request
    validate_request(&category_request)?;

    let inventory_service = InventoryService::new(db);
    let category = inventory_service.create_category(category_request).await?;

    Ok(Json(category))
}

pub async fn update_category(
    State(db): State<Database>,
    auth: AuthContext,
    Path(id): Path<i32>,
    Json(category_request): Json<UpdateCategoryRequest>,
) -> AppResult<Json<Category>> {
    auth.require_permission(Permission::UpdateCategory)?;

    // Validate request
    validate_request(&category_request)?;

    let inventory_service = InventoryService::new(db);
    let category = inventory_service
        .update_category(id, category_request)
        .await?;

    Ok(Json(category))
}

pub async fn delete_category(
    State(db): State<Database>,
    auth: AuthContext,
    Path(id): Path<i32>,
) -> AppResult<Json<HashMap<String, String>>> {
    auth.require_permission(Permission::DeleteCategory)?;

    let inventory_service = InventoryService::new(db);
    inventory_service.delete_category(id).await?;

    let mut response = HashMap::new();
    response.insert(
        "message".to_string(),
        "Category deleted successfully".to_string(),
    );
    Ok(Json(response))
}
