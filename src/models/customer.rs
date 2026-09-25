use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Customer {
    pub id: i32,
    pub name: String,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub address: Option<String>,
    pub total_spent: Decimal,
    pub loyalty_points: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateCustomerRequest {
    #[validate(length(min = 1, max = 100))]
    pub name: String,
    #[validate(length(max = 20))]
    pub phone: Option<String>,
    #[validate(email)]
    pub email: Option<String>,
    pub address: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateCustomerRequest {
    #[validate(length(min = 1, max = 100))]
    pub name: Option<String>,
    #[validate(length(max = 20))]
    pub phone: Option<String>,
    #[validate(email)]
    pub email: Option<String>,
    pub address: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CustomerResponse {
    pub id: i32,
    pub name: String,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub address: Option<String>,
    pub total_spent: Decimal,
    pub loyalty_points: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<Customer> for CustomerResponse {
    fn from(customer: Customer) -> Self {
        CustomerResponse {
            id: customer.id,
            name: customer.name,
            phone: customer.phone,
            email: customer.email,
            address: customer.address,
            total_spent: customer.total_spent,
            loyalty_points: customer.loyalty_points,
            created_at: customer.created_at,
            updated_at: customer.updated_at,
        }
    }
}
