use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Branch {
    pub id: i32,
    pub name: String,
    pub code: String,
    pub address: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub manager_name: Option<String>,
    pub manager_phone: Option<String>,
    pub tax_id: Option<String>,
    pub business_license: Option<String>,
    pub opening_time: Option<chrono::NaiveTime>,
    pub closing_time: Option<chrono::NaiveTime>,
    pub is_active: bool,
    pub is_main_branch: bool,
    pub settings: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateBranchRequest {
    pub name: String,
    pub code: String,
    pub address: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub manager_name: Option<String>,
    pub manager_phone: Option<String>,
    pub tax_id: Option<String>,
    pub business_license: Option<String>,
    pub opening_time: Option<String>,
    pub closing_time: Option<String>,
    pub is_main_branch: Option<bool>,
    pub settings: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateBranchRequest {
    pub name: Option<String>,
    pub address: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub manager_name: Option<String>,
    pub manager_phone: Option<String>,
    pub tax_id: Option<String>,
    pub business_license: Option<String>,
    pub opening_time: Option<String>,
    pub closing_time: Option<String>,
    pub is_active: Option<bool>,
    pub is_main_branch: Option<bool>,
    pub settings: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
pub struct BranchResponse {
    pub id: i32,
    pub name: String,
    pub code: String,
    pub address: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub manager_name: Option<String>,
    pub manager_phone: Option<String>,
    pub tax_id: Option<String>,
    pub business_license: Option<String>,
    pub opening_time: Option<String>,
    pub closing_time: Option<String>,
    pub is_active: bool,
    pub is_main_branch: bool,
    pub settings: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<Branch> for BranchResponse {
    fn from(branch: Branch) -> Self {
        BranchResponse {
            id: branch.id,
            name: branch.name,
            code: branch.code,
            address: branch.address,
            phone: branch.phone,
            email: branch.email,
            manager_name: branch.manager_name,
            manager_phone: branch.manager_phone,
            tax_id: branch.tax_id,
            business_license: branch.business_license,
            opening_time: branch.opening_time.map(|t| t.format("%H:%M").to_string()),
            closing_time: branch.closing_time.map(|t| t.format("%H:%M").to_string()),
            is_active: branch.is_active,
            is_main_branch: branch.is_main_branch,
            settings: branch.settings,
            created_at: branch.created_at,
            updated_at: branch.updated_at,
        }
    }
}

#[derive(Debug, Serialize, FromRow)]
pub struct BranchPerformance {
    pub branch_id: i32,
    pub branch_name: String,
    pub branch_code: String,
    pub total_sales: i64,
    pub total_revenue: rust_decimal::Decimal,
    pub average_order_value: rust_decimal::Decimal,
    pub active_cashiers: i64,
    pub total_customers: i64,
    pub total_products: i64,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, FromRow)]
pub struct BranchComparison {
    pub branch_id: i32,
    pub branch_name: String,
    pub current_revenue: rust_decimal::Decimal,
    pub previous_revenue: rust_decimal::Decimal,
    pub revenue_growth_percentage: rust_decimal::Decimal,
    pub current_sales: i64,
    pub previous_sales: i64,
    pub sales_growth_percentage: rust_decimal::Decimal,
}

#[derive(Debug, Serialize, FromRow)]
pub struct BranchInventorySummary {
    pub branch_id: i32,
    pub branch_name: String,
    pub total_products: i64,
    pub low_stock_products: i64,
    pub out_of_stock_products: i64,
    pub total_inventory_value: rust_decimal::Decimal,
    pub total_cost_value: rust_decimal::Decimal,
}

#[derive(Debug, Serialize, FromRow)]
pub struct UserBranchAccess {
    pub user_id: i32,
    pub username: String,
    pub full_name: String,
    pub role: crate::models::UserRole,
    pub can_access_all_branches: bool,
    pub accessible_branch_ids: Vec<i32>,
    pub accessible_branch_names: Vec<String>,
    pub primary_branch_name: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct AssignUserToBranchRequest {
    pub user_id: i32,
    pub branch_id: i32,
    pub is_primary: Option<bool>,
}

#[derive(Debug, Serialize, FromRow)]
pub struct UserBranchAssignment {
    pub id: i32,
    pub user_id: i32,
    pub branch_id: i32,
    pub assigned_at: DateTime<Utc>,
    pub assigned_by: Option<i32>,
    pub is_primary: bool,
}
