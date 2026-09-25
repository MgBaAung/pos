use crate::error::AppError;
use crate::restaurant::domain::kitchen_orders::{
    CreateKitchenOrderRequest, KotStatus, UpdateKitchenOrderItemRequest, UpdateKitchenOrderRequest,
};
use crate::restaurant::routes::RestaurantAppState;
use crate::restaurant::services::KitchenOrderService;
use axum::{
    extract::{Path, State},
    Json,
};

pub async fn create_kitchen_order(
    State(app_state): State<RestaurantAppState>,
    Json(request): Json<CreateKitchenOrderRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let kitchen_order_service =
        KitchenOrderService::new(&app_state.database, app_state.kitchen_event_sender);
    let kitchen_order = kitchen_order_service.create_kitchen_order(request).await?;

    Ok(Json(serde_json::json!({
        "success": true,
        "data": kitchen_order
    })))
}

pub async fn get_kitchen_order(
    State(app_state): State<RestaurantAppState>,
    Path(id): Path<i32>,
) -> Result<Json<serde_json::Value>, AppError> {
    let kitchen_order_service =
        KitchenOrderService::new(&app_state.database, app_state.kitchen_event_sender);
    let kitchen_order = kitchen_order_service.get_kitchen_order(id).await?;

    Ok(Json(serde_json::json!({
        "success": true,
        "data": kitchen_order
    })))
}

pub async fn list_kitchen_orders(
    State(app_state): State<RestaurantAppState>,
    axum::extract::Query(params): axum::extract::Query<KitchenOrderQueryParams>,
) -> Result<Json<serde_json::Value>, AppError> {
    let kitchen_order_service =
        KitchenOrderService::new(&app_state.database, app_state.kitchen_event_sender);
    let kitchen_orders = kitchen_order_service
        .list_kitchen_orders(params.status, params.table_id)
        .await?;

    Ok(Json(serde_json::json!({
        "success": true,
        "data": kitchen_orders,
        "count": kitchen_orders.len()
    })))
}

pub async fn update_kitchen_order(
    State(app_state): State<RestaurantAppState>,
    Path(id): Path<i32>,
    Json(request): Json<UpdateKitchenOrderRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let kitchen_order_service =
        KitchenOrderService::new(&app_state.database, app_state.kitchen_event_sender);
    let kitchen_order = kitchen_order_service
        .update_kitchen_order(id, request)
        .await?;

    Ok(Json(serde_json::json!({
        "success": true,
        "data": kitchen_order
    })))
}

pub async fn update_kitchen_order_item(
    State(app_state): State<RestaurantAppState>,
    Path(id): Path<i32>,
    Json(request): Json<UpdateKitchenOrderItemRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let kitchen_order_service =
        KitchenOrderService::new(&app_state.database, app_state.kitchen_event_sender);
    let item = kitchen_order_service
        .update_kitchen_order_item(id, request)
        .await?;

    Ok(Json(serde_json::json!({
        "success": true,
        "data": item
    })))
}

pub async fn get_pending_kitchen_orders(
    State(app_state): State<RestaurantAppState>,
) -> Result<Json<serde_json::Value>, AppError> {
    let kitchen_order_service =
        KitchenOrderService::new(&app_state.database, app_state.kitchen_event_sender);
    let kitchen_orders = kitchen_order_service.get_pending_kitchen_orders().await?;

    Ok(Json(serde_json::json!({
        "success": true,
        "data": kitchen_orders,
        "count": kitchen_orders.len()
    })))
}

pub async fn get_active_kitchen_orders(
    State(app_state): State<RestaurantAppState>,
) -> Result<Json<serde_json::Value>, AppError> {
    let kitchen_order_service =
        KitchenOrderService::new(&app_state.database, app_state.kitchen_event_sender);
    let kitchen_orders = kitchen_order_service.get_active_kitchen_orders().await?;

    Ok(Json(serde_json::json!({
        "success": true,
        "data": kitchen_orders,
        "count": kitchen_orders.len()
    })))
}

#[derive(serde::Deserialize)]
pub struct KitchenOrderQueryParams {
    status: Option<KotStatus>,
    table_id: Option<i32>,
}
