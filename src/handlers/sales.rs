use crate::database::Database;
use crate::error::AppResult;
use crate::middleware::auth::AuthContext;
use crate::middleware::rbac::Permission;
use crate::models::{
    CloseCashDrawerSessionRequest, CreateCashDrawerSessionRequest, CreateSaleRequest, ReceiptData,
    Sale, SaleResponse, SaleStatus,
};
use crate::sales::SalesService;
use crate::utils::validate_request;
use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ListSalesQuery {
    pub cashier_id: Option<i32>,
    pub customer_id: Option<i32>,
    pub status: Option<SaleStatus>,
    pub payment_method: Option<crate::models::PaymentMethod>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

pub async fn list_sales(
    State(db): State<Database>,
    Query(params): Query<ListSalesQuery>,
) -> AppResult<Json<Vec<Sale>>> {
    let sales_service = SalesService::new(db);
    let sales = sales_service
        .list_sales(
            params.cashier_id,
            params.customer_id,
            params.status,
            params.payment_method,
            params.limit.unwrap_or(50),
            params.offset.unwrap_or(0),
        )
        .await?;

    Ok(Json(sales))
}

pub async fn get_sale(
    State(db): State<Database>,
    Path(id): Path<i32>,
) -> AppResult<Json<SaleResponse>> {
    let sales_service = SalesService::new(db);
    let sale = sales_service.get_sale(id).await?;
    let items = sales_service.get_sale_items(id).await?;

    let sale_response = SaleResponse::new(sale, items);

    Ok(Json(sale_response))
}

pub async fn get_sale_by_invoice(
    State(db): State<Database>,
    Path(invoice_no): Path<String>,
) -> AppResult<Json<SaleResponse>> {
    let sales_service = SalesService::new(db);
    let sale = sales_service.get_sale_by_invoice(&invoice_no).await?;
    let items = sales_service.get_sale_items(sale.id).await?;

    let sale_response = SaleResponse::new(sale, items);

    Ok(Json(sale_response))
}

pub async fn create_sale(
    State(db): State<Database>,
    auth: AuthContext,
    Json(sale_request): Json<CreateSaleRequest>,
) -> AppResult<Json<SaleResponse>> {
    // Validate request
    validate_request(&sale_request)?;

    let sales_service = SalesService::new(db);
    let sale = sales_service
        .create_sale(sale_request, auth.user_id)
        .await?;

    let items = sales_service.get_sale_items(sale.id).await?;

    let sale_response = SaleResponse::new(sale, items);

    Ok(Json(sale_response))
}

pub async fn refund_sale(
    State(db): State<Database>,
    auth: AuthContext,
    Path(id): Path<i32>,
) -> AppResult<Json<SaleResponse>> {
    auth.require_permission(Permission::RefundSale)?;

    let sales_service = SalesService::new(db);
    let sale = sales_service.refund_sale(id, auth.user_id).await?;

    let items = sales_service.get_sale_items(sale.id).await?;

    let sale_response = SaleResponse::new(sale, items);

    Ok(Json(sale_response))
}

pub async fn generate_receipt(
    State(db): State<Database>,
    Path(id): Path<i32>,
) -> AppResult<Json<ReceiptData>> {
    let sales_service = SalesService::new(db);
    let receipt = sales_service.generate_receipt(id).await?;
    Ok(Json(receipt))
}

/// Render a sale's receipt as raw ESC/POS bytes for a thermal printer.
pub async fn generate_escpos_receipt(
    State(db): State<Database>,
    Path(id): Path<i32>,
) -> AppResult<impl axum::response::IntoResponse> {
    let sales_service = SalesService::new(db);
    let receipt = sales_service.generate_receipt(id).await?;
    let bytes = crate::printer::render_escpos(&receipt);
    let disposition = format!("attachment; filename=\"receipt-{}.bin\"", id);

    Ok((
        [
            (
                axum::http::header::CONTENT_TYPE,
                "application/octet-stream".to_string(),
            ),
            (axum::http::header::CONTENT_DISPOSITION, disposition),
        ],
        bytes,
    ))
}

pub async fn open_cash_drawer(
    State(db): State<Database>,
    auth: AuthContext,
    Json(drawer_request): Json<CreateCashDrawerSessionRequest>,
) -> AppResult<Json<crate::models::CashDrawerSession>> {
    let sales_service = SalesService::new(db);
    let session = sales_service
        .open_cash_drawer(drawer_request, auth.user_id)
        .await?;

    Ok(Json(session))
}

pub async fn close_cash_drawer(
    State(db): State<Database>,
    Path(id): Path<i32>,
    Json(drawer_request): Json<CloseCashDrawerSessionRequest>,
) -> AppResult<Json<crate::models::CashDrawerSession>> {
    let sales_service = SalesService::new(db);
    let session = sales_service.close_cash_drawer(id, drawer_request).await?;

    Ok(Json(session))
}

pub async fn get_cash_drawer_sessions(
    State(db): State<Database>,
    auth: AuthContext,
) -> AppResult<Json<Vec<crate::models::CashDrawerSession>>> {
    let sales_service = SalesService::new(db);
    let sessions = sales_service.get_cash_drawer_sessions(auth.user_id).await?;
    Ok(Json(sessions))
}
