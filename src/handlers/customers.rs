use crate::customers::CustomerService;
use crate::database::Database;
use crate::error::AppResult;
use crate::middleware::auth::AuthContext;
use crate::middleware::rbac::Permission;
use crate::models::{CreateCustomerRequest, CustomerResponse, UpdateCustomerRequest};
use crate::utils::validate_request;
use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;
use validator::Validate;

#[derive(Debug, Deserialize)]
pub struct ListCustomersQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

pub async fn list_customers(
    State(db): State<Database>,
    Query(params): Query<ListCustomersQuery>,
) -> AppResult<Json<Vec<CustomerResponse>>> {
    let customer_service = CustomerService::new(db);
    let customers = customer_service
        .list_customers(params.limit.unwrap_or(50), params.offset.unwrap_or(0))
        .await?;

    let responses: Vec<CustomerResponse> =
        customers.into_iter().map(CustomerResponse::from).collect();
    Ok(Json(responses))
}

pub async fn get_customer(
    State(db): State<Database>,
    Path(id): Path<i32>,
) -> AppResult<Json<CustomerResponse>> {
    let customer_service = CustomerService::new(db);
    let customer = customer_service.get_customer(id).await?;
    Ok(Json(CustomerResponse::from(customer)))
}

pub async fn get_customer_by_phone(
    State(db): State<Database>,
    Path(phone): Path<String>,
) -> AppResult<Json<CustomerResponse>> {
    let customer_service = CustomerService::new(db);
    let customer = customer_service.get_customer_by_phone(&phone).await?;
    Ok(Json(CustomerResponse::from(customer)))
}

pub async fn search_customers(
    State(db): State<Database>,
    Path(query): Path<String>,
) -> AppResult<Json<Vec<CustomerResponse>>> {
    let customer_service = CustomerService::new(db);
    let customers = customer_service.search_customers(&query).await?;

    let responses: Vec<CustomerResponse> =
        customers.into_iter().map(CustomerResponse::from).collect();
    Ok(Json(responses))
}

pub async fn create_customer(
    State(db): State<Database>,
    Json(customer_request): Json<CreateCustomerRequest>,
) -> AppResult<Json<CustomerResponse>> {
    // Validate request
    validate_request(&customer_request)?;

    let customer_service = CustomerService::new(db);
    let customer = customer_service.create_customer(customer_request).await?;

    Ok(Json(CustomerResponse::from(customer)))
}

pub async fn update_customer(
    State(db): State<Database>,
    Path(id): Path<i32>,
    Json(customer_request): Json<UpdateCustomerRequest>,
) -> AppResult<Json<CustomerResponse>> {
    // Validate request
    validate_request(&customer_request)?;

    let customer_service = CustomerService::new(db);
    let customer = customer_service
        .update_customer(id, customer_request)
        .await?;

    Ok(Json(CustomerResponse::from(customer)))
}

pub async fn delete_customer(
    State(db): State<Database>,
    Path(id): Path<i32>,
) -> AppResult<Json<std::collections::HashMap<String, String>>> {
    let customer_service = CustomerService::new(db);
    customer_service.delete_customer(id).await?;

    let mut response = std::collections::HashMap::new();
    response.insert(
        "message".to_string(),
        "Customer deleted successfully".to_string(),
    );
    Ok(Json(response))
}

pub async fn get_customer_purchase_history(
    State(db): State<Database>,
    Path(id): Path<i32>,
) -> AppResult<Json<Vec<crate::models::Sale>>> {
    let customer_service = CustomerService::new(db);
    let history = customer_service
        .get_customer_purchase_history(id, 50)
        .await?;
    Ok(Json(history))
}

#[derive(Debug, Deserialize, Validate)]
pub struct LoyaltyPointsRequest {
    #[validate(range(min = 1))]
    pub points: i32,
}

#[derive(Debug, Deserialize)]
pub struct TopCustomersQuery {
    pub limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct LoyaltyPointsQuery {
    pub min_points: Option<i32>,
}

/// Add loyalty points to a customer (Cashier+).
pub async fn add_loyalty_points(
    State(db): State<Database>,
    auth: AuthContext,
    Path(id): Path<i32>,
    Json(request): Json<LoyaltyPointsRequest>,
) -> AppResult<Json<CustomerResponse>> {
    auth.require_permission(Permission::UpdateCustomer)?;
    validate_request(&request)?;

    let customer_service = CustomerService::new(db);
    let customer = customer_service
        .add_loyalty_points(id, request.points)
        .await?;
    Ok(Json(CustomerResponse::from(customer)))
}

/// Redeem loyalty points from a customer (Cashier+).
pub async fn redeem_loyalty_points(
    State(db): State<Database>,
    auth: AuthContext,
    Path(id): Path<i32>,
    Json(request): Json<LoyaltyPointsRequest>,
) -> AppResult<Json<CustomerResponse>> {
    auth.require_permission(Permission::UpdateCustomer)?;
    validate_request(&request)?;

    let customer_service = CustomerService::new(db);
    let customer = customer_service
        .redeem_loyalty_points(id, request.points)
        .await?;
    Ok(Json(CustomerResponse::from(customer)))
}

/// Top customers by total spend (Manager+).
pub async fn get_top_customers(
    State(db): State<Database>,
    auth: AuthContext,
    Query(params): Query<TopCustomersQuery>,
) -> AppResult<Json<Vec<CustomerResponse>>> {
    auth.require_permission(Permission::ViewReports)?;

    let customer_service = CustomerService::new(db);
    let customers = customer_service
        .get_top_customers(params.limit.unwrap_or(10))
        .await?;

    let responses: Vec<CustomerResponse> =
        customers.into_iter().map(CustomerResponse::from).collect();
    Ok(Json(responses))
}

/// Customers with at least `min_points` loyalty points (Cashier+).
pub async fn get_customers_by_loyalty_points(
    State(db): State<Database>,
    auth: AuthContext,
    Query(params): Query<LoyaltyPointsQuery>,
) -> AppResult<Json<Vec<CustomerResponse>>> {
    auth.require_permission(Permission::ViewCustomers)?;

    let customer_service = CustomerService::new(db);
    let customers = customer_service
        .get_customers_by_loyalty_points(params.min_points.unwrap_or(1))
        .await?;

    let responses: Vec<CustomerResponse> =
        customers.into_iter().map(CustomerResponse::from).collect();
    Ok(Json(responses))
}
