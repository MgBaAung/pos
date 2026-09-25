use crate::database::Database;
use crate::error::AppResult;
use crate::inventory::InventoryService;
use crate::middleware::auth::AuthContext;
use crate::middleware::rbac::Permission;
use crate::models::{
    CreateProductRequest, CreateProductVariantRequest, InventoryLog, ProductResponse,
    ProductVariantResponse, StockAdjustmentRequest, UpdateProductRequest,
    UpdateProductVariantRequest,
};
use crate::utils::{validate_request, BranchContext};
use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
pub struct ListProductsQuery {
    pub category_id: Option<i32>,
    pub active_only: Option<bool>,
    pub low_stock_only: Option<bool>,
}

pub async fn list_products(
    State(db): State<Database>,
    Query(params): Query<ListProductsQuery>,
) -> AppResult<Json<Vec<ProductResponse>>> {
    let inventory_service = InventoryService::new(db);
    let products = inventory_service
        .list_products(
            params.category_id,
            params.active_only.unwrap_or(true),
            params.low_stock_only.unwrap_or(false),
        )
        .await?;

    let responses: Vec<ProductResponse> = products.into_iter().map(ProductResponse::from).collect();
    Ok(Json(responses))
}

pub async fn get_product(
    State(db): State<Database>,
    Path(id): Path<i32>,
) -> AppResult<Json<ProductResponse>> {
    let inventory_service = InventoryService::new(db);
    let product = inventory_service.get_product(id).await?;
    Ok(Json(ProductResponse::from(product)))
}

pub async fn get_product_by_sku(
    State(db): State<Database>,
    Path(sku): Path<String>,
) -> AppResult<Json<ProductResponse>> {
    let inventory_service = InventoryService::new(db);
    let product = inventory_service.get_product_by_sku(&sku).await?;
    Ok(Json(ProductResponse::from(product)))
}

pub async fn get_product_by_barcode(
    State(db): State<Database>,
    Path(barcode): Path<String>,
) -> AppResult<Json<ProductResponse>> {
    let inventory_service = InventoryService::new(db);
    let product = inventory_service.get_product_by_barcode(&barcode).await?;
    Ok(Json(ProductResponse::from(product)))
}

pub async fn create_product(
    State(db): State<Database>,
    auth: AuthContext,
    branch_context: BranchContext,
    Json(product_request): Json<CreateProductRequest>,
) -> AppResult<Json<ProductResponse>> {
    auth.require_permission(Permission::CreateProduct)?;

    // Validate request
    validate_request(&product_request)?;

    let inventory_service = InventoryService::new(db);
    let product = inventory_service
        .create_product(product_request, auth.user_id, &branch_context)
        .await?;

    Ok(Json(ProductResponse::from(product)))
}

pub async fn update_product(
    State(db): State<Database>,
    auth: AuthContext,
    Path(id): Path<i32>,
    Json(product_request): Json<UpdateProductRequest>,
) -> AppResult<Json<ProductResponse>> {
    auth.require_permission(Permission::UpdateProduct)?;

    // Validate request
    validate_request(&product_request)?;

    let inventory_service = InventoryService::new(db);
    let product = inventory_service
        .update_product(id, product_request)
        .await?;

    Ok(Json(ProductResponse::from(product)))
}

pub async fn delete_product(
    State(db): State<Database>,
    auth: AuthContext,
    Path(id): Path<i32>,
) -> AppResult<Json<HashMap<String, String>>> {
    auth.require_permission(Permission::DeleteProduct)?;

    let inventory_service = InventoryService::new(db);
    inventory_service.delete_product(id).await?;

    let mut response = HashMap::new();
    response.insert(
        "message".to_string(),
        "Product deleted successfully".to_string(),
    );
    Ok(Json(response))
}

pub async fn adjust_stock(
    State(db): State<Database>,
    auth: AuthContext,
    Path(id): Path<i32>,
    Json(adjustment_request): Json<StockAdjustmentRequest>,
) -> AppResult<Json<ProductResponse>> {
    auth.require_permission(Permission::AdjustStock)?;

    let inventory_service = InventoryService::new(db);
    let product = inventory_service
        .adjust_stock(id, adjustment_request, auth.user_id)
        .await?;

    Ok(Json(ProductResponse::from(product)))
}

pub async fn get_low_stock_products(
    State(db): State<Database>,
) -> AppResult<Json<Vec<ProductResponse>>> {
    let inventory_service = InventoryService::new(db);
    let products = inventory_service.get_low_stock_products().await?;

    let responses: Vec<ProductResponse> = products.into_iter().map(ProductResponse::from).collect();
    Ok(Json(responses))
}

// ==================== Product Variants ====================

pub async fn list_product_variants(
    State(db): State<Database>,
    auth: AuthContext,
    Path(product_id): Path<i32>,
) -> AppResult<Json<Vec<ProductVariantResponse>>> {
    auth.require_permission(Permission::ViewProducts)?;

    let inventory_service = InventoryService::new(db);
    let variants = inventory_service.list_variants(product_id).await?;

    let responses: Vec<ProductVariantResponse> = variants
        .into_iter()
        .map(ProductVariantResponse::from)
        .collect();
    Ok(Json(responses))
}

pub async fn create_product_variant(
    State(db): State<Database>,
    auth: AuthContext,
    Path(product_id): Path<i32>,
    Json(request): Json<CreateProductVariantRequest>,
) -> AppResult<Json<ProductVariantResponse>> {
    auth.require_permission(Permission::CreateProduct)?;
    validate_request(&request)?;

    let inventory_service = InventoryService::new(db);
    let variant = inventory_service
        .create_variant(product_id, request)
        .await?;

    Ok(Json(ProductVariantResponse::from(variant)))
}

pub async fn get_product_variant(
    State(db): State<Database>,
    auth: AuthContext,
    Path(id): Path<i32>,
) -> AppResult<Json<ProductVariantResponse>> {
    auth.require_permission(Permission::ViewProducts)?;

    let inventory_service = InventoryService::new(db);
    let variant = inventory_service.get_variant(id).await?;

    Ok(Json(ProductVariantResponse::from(variant)))
}

pub async fn update_product_variant(
    State(db): State<Database>,
    auth: AuthContext,
    Path(id): Path<i32>,
    Json(request): Json<UpdateProductVariantRequest>,
) -> AppResult<Json<ProductVariantResponse>> {
    auth.require_permission(Permission::UpdateProduct)?;
    validate_request(&request)?;

    let inventory_service = InventoryService::new(db);
    let variant = inventory_service.update_variant(id, request).await?;

    Ok(Json(ProductVariantResponse::from(variant)))
}

pub async fn delete_product_variant(
    State(db): State<Database>,
    auth: AuthContext,
    Path(id): Path<i32>,
) -> AppResult<Json<HashMap<String, String>>> {
    auth.require_permission(Permission::DeleteProduct)?;

    let inventory_service = InventoryService::new(db);
    inventory_service.delete_variant(id).await?;

    let mut response = HashMap::new();
    response.insert(
        "message".to_string(),
        "Product variant deleted successfully".to_string(),
    );
    Ok(Json(response))
}

#[derive(Debug, Deserialize)]
pub struct RecentInventoryLogsQuery {
    pub limit: Option<i64>,
}

/// Stock movement history for a single product (Manager+).
pub async fn get_product_inventory_logs(
    State(db): State<Database>,
    auth: AuthContext,
    Path(id): Path<i32>,
) -> AppResult<Json<Vec<InventoryLog>>> {
    auth.require_permission(Permission::ViewInventoryLogs)?;

    let inventory_service = InventoryService::new(db);
    let logs = inventory_service.get_inventory_logs(id).await?;
    Ok(Json(logs))
}

/// Most recent stock movements across all products (Manager+).
pub async fn get_recent_inventory_logs(
    State(db): State<Database>,
    auth: AuthContext,
    Query(params): Query<RecentInventoryLogsQuery>,
) -> AppResult<Json<Vec<InventoryLog>>> {
    auth.require_permission(Permission::ViewInventoryLogs)?;

    let inventory_service = InventoryService::new(db);
    let logs = inventory_service
        .get_recent_inventory_logs(params.limit.unwrap_or(50))
        .await?;
    Ok(Json(logs))
}
