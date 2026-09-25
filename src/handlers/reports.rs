use crate::database::Database;
use crate::error::{AppError, AppResult};
use crate::middleware::auth::extract_auth_context;
use crate::middleware::rbac::Permission;
use crate::reports::{DailySalesReport, MonthlySalesReport, ReportService, TopSellingProduct};
use axum::{
    extract::{Path, Query, Request, State},
    Json,
};
use chrono::NaiveDate;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct DateRangeQuery {
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
}

pub async fn daily_sales_report(
    State(db): State<Database>,
    Path(date): Path<NaiveDate>,
    request: Request,
) -> AppResult<Json<DailySalesReport>> {
    // Check permissions
    let auth_context = extract_auth_context(&request)?;
    if !Permission::ViewReports.check(auth_context.role) {
        return Err(AppError::Authorization(
            "Insufficient permissions to view reports".to_string(),
        ));
    }

    let report_service = ReportService::new(db);
    let report = report_service.daily_sales_report(date).await?;
    Ok(Json(report))
}

pub async fn today_sales_report(
    State(db): State<Database>,
    request: Request,
) -> AppResult<Json<DailySalesReport>> {
    // Check permissions
    let auth_context = extract_auth_context(&request)?;
    if !Permission::ViewReports.check(auth_context.role) {
        return Err(AppError::Authorization(
            "Insufficient permissions to view reports".to_string(),
        ));
    }

    let report_service = ReportService::new(db);
    let report = report_service.today_sales_report().await?;
    Ok(Json(report))
}

pub async fn monthly_sales_report(
    State(db): State<Database>,
    Path((year, month)): Path<(i32, i32)>,
    request: Request,
) -> AppResult<Json<MonthlySalesReport>> {
    // Check permissions
    let auth_context = extract_auth_context(&request)?;
    if !Permission::ViewReports.check(auth_context.role) {
        return Err(AppError::Authorization(
            "Insufficient permissions to view reports".to_string(),
        ));
    }

    let report_service = ReportService::new(db);
    let report = report_service.monthly_sales_report(year, month).await?;
    Ok(Json(report))
}

pub async fn current_month_report(
    State(db): State<Database>,
    request: Request,
) -> AppResult<Json<MonthlySalesReport>> {
    // Check permissions
    let auth_context = extract_auth_context(&request)?;
    if !Permission::ViewReports.check(auth_context.role) {
        return Err(AppError::Authorization(
            "Insufficient permissions to view reports".to_string(),
        ));
    }

    let report_service = ReportService::new(db);
    let report = report_service.current_month_report().await?;
    Ok(Json(report))
}

pub async fn top_selling_products(
    State(db): State<Database>,
    Query(params): Query<DateRangeQuery>,
    request: Request,
) -> AppResult<Json<Vec<TopSellingProduct>>> {
    // Check permissions
    let auth_context = extract_auth_context(&request)?;
    if !Permission::ViewReports.check(auth_context.role) {
        return Err(AppError::Authorization(
            "Insufficient permissions to view reports".to_string(),
        ));
    }

    let report_service = ReportService::new(db);

    let products = if let (Some(start), Some(end)) = (params.start_date, params.end_date) {
        report_service.top_selling_products(start, end, 20).await?
    } else {
        report_service.all_time_top_selling_products(20).await?
    };

    Ok(Json(products))
}

pub async fn low_stock_report(
    State(db): State<Database>,
    request: Request,
) -> AppResult<Json<Vec<crate::models::ProductResponse>>> {
    // Check permissions
    let auth_context = extract_auth_context(&request)?;
    if !Permission::ViewInventoryLogs.check(auth_context.role) {
        return Err(AppError::Authorization(
            "Insufficient permissions to view inventory reports".to_string(),
        ));
    }

    let report_service = ReportService::new(db);
    let products = report_service.low_stock_report().await?;

    let responses: Vec<crate::models::ProductResponse> =
        products.into_iter().map(|p| p.into()).collect();
    Ok(Json(responses))
}

pub async fn revenue_summary(
    State(db): State<Database>,
    Query(params): Query<DateRangeQuery>,
    request: Request,
) -> AppResult<Json<crate::reports::RevenueSummary>> {
    // Check permissions
    let auth_context = extract_auth_context(&request)?;
    if !Permission::ViewFinancialReports.check(auth_context.role) {
        return Err(AppError::Authorization(
            "Insufficient permissions to view financial reports".to_string(),
        ));
    }

    let start_date = params
        .start_date
        .unwrap_or_else(|| chrono::Utc::now().naive_utc().date() - chrono::Duration::days(30));
    let end_date = params
        .end_date
        .unwrap_or_else(|| chrono::Utc::now().naive_utc().date());

    let report_service = ReportService::new(db);
    let summary = report_service.revenue_summary(start_date, end_date).await?;
    Ok(Json(summary))
}

pub async fn sales_trend(
    State(db): State<Database>,
    Query(params): Query<DateRangeQuery>,
    request: Request,
) -> AppResult<Json<Vec<(NaiveDate, rust_decimal::Decimal)>>> {
    // Check permissions
    let auth_context = extract_auth_context(&request)?;
    if !Permission::ViewReports.check(auth_context.role) {
        return Err(AppError::Authorization(
            "Insufficient permissions to view reports".to_string(),
        ));
    }

    let start_date = params
        .start_date
        .unwrap_or_else(|| chrono::Utc::now().naive_utc().date() - chrono::Duration::days(30));
    let end_date = params
        .end_date
        .unwrap_or_else(|| chrono::Utc::now().naive_utc().date());

    let report_service = ReportService::new(db);
    let trend = report_service.sales_trend(start_date, end_date).await?;
    Ok(Json(trend))
}
