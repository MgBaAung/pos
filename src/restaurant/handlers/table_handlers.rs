use crate::error::AppError;
use crate::restaurant::domain::tables::{CreateTableRequest, TableStatus, UpdateTableRequest};
use crate::restaurant::routes::RestaurantAppState;
use crate::restaurant::services::TableService;
use axum::{
    extract::{Path, State},
    Json,
};
use validator::Validate;

pub async fn create_table(
    State(app_state): State<RestaurantAppState>,
    Json(request): Json<CreateTableRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    request
        .validate()
        .map_err(|e| AppError::BadRequest(e.to_string()))?;

    let table_service = TableService::new(&app_state.database);
    let table = table_service.create_table(request).await?;

    Ok(Json(serde_json::json!({
        "success": true,
        "data": table
    })))
}

pub async fn get_table(
    State(app_state): State<RestaurantAppState>,
    Path(id): Path<i32>,
) -> Result<Json<serde_json::Value>, AppError> {
    let table_service = TableService::new(&app_state.database);
    let table = table_service.get_table(id).await?;

    Ok(Json(serde_json::json!({
        "success": true,
        "data": table
    })))
}

pub async fn get_table_by_number(
    State(app_state): State<RestaurantAppState>,
    Path(table_number): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let table_service = TableService::new(&app_state.database);
    let table = table_service.get_table_by_number(&table_number).await?;

    Ok(Json(serde_json::json!({
        "success": true,
        "data": table
    })))
}

pub async fn list_tables(
    State(app_state): State<RestaurantAppState>,
    axum::extract::Query(params): axum::extract::Query<TableQueryParams>,
) -> Result<Json<serde_json::Value>, AppError> {
    let table_service = TableService::new(&app_state.database);
    let tables = table_service
        .list_tables(params.status, params.zone, params.is_active)
        .await?;

    Ok(Json(serde_json::json!({
        "success": true,
        "data": tables,
        "count": tables.len()
    })))
}

pub async fn update_table(
    State(app_state): State<RestaurantAppState>,
    Path(id): Path<i32>,
    Json(request): Json<UpdateTableRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    request
        .validate()
        .map_err(|e| AppError::BadRequest(e.to_string()))?;

    let table_service = TableService::new(&app_state.database);
    let table = table_service.update_table(id, request).await?;

    Ok(Json(serde_json::json!({
        "success": true,
        "data": table
    })))
}

pub async fn delete_table(
    State(app_state): State<RestaurantAppState>,
    Path(id): Path<i32>,
) -> Result<Json<serde_json::Value>, AppError> {
    let table_service = TableService::new(&app_state.database);
    table_service.delete_table(id).await?;

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Table deleted successfully"
    })))
}

pub async fn get_available_tables(
    State(app_state): State<RestaurantAppState>,
    axum::extract::Query(params): axum::extract::Query<AvailableTableParams>,
) -> Result<Json<serde_json::Value>, AppError> {
    let table_service = TableService::new(&app_state.database);
    let tables = table_service
        .get_available_tables(params.guest_count)
        .await?;

    Ok(Json(serde_json::json!({
        "success": true,
        "data": tables,
        "count": tables.len()
    })))
}

pub async fn update_table_status(
    State(app_state): State<RestaurantAppState>,
    Path(id): Path<i32>,
    axum::extract::Query(params): axum::extract::Query<UpdateTableStatusParams>,
) -> Result<Json<serde_json::Value>, AppError> {
    let table_service = TableService::new(&app_state.database);
    let table = table_service.update_table_status(id, params.status).await?;

    Ok(Json(serde_json::json!({
        "success": true,
        "data": table
    })))
}

#[derive(serde::Deserialize)]
pub struct TableQueryParams {
    status: Option<TableStatus>,
    zone: Option<String>,
    is_active: Option<bool>,
}

#[derive(serde::Deserialize)]
pub struct AvailableTableParams {
    guest_count: Option<i32>,
}

#[derive(serde::Deserialize)]
pub struct UpdateTableStatusParams {
    pub status: TableStatus,
}
