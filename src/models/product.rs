use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Product {
    pub id: i32,
    pub sku: String,
    pub barcode: Option<String>,
    pub name: String,
    pub description: Option<String>,
    pub category_id: Option<i32>,
    pub cost_price: Decimal,
    pub selling_price: Decimal,
    pub stock_quantity: i32,
    pub low_stock_threshold: i32,
    pub tax_rate: Decimal,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateProductRequest {
    #[validate(length(min = 1, max = 100))]
    pub sku: String,
    #[validate(length(max = 100))]
    pub barcode: Option<String>,
    #[validate(length(min = 1, max = 150))]
    pub name: String,
    pub description: Option<String>,
    pub category_id: Option<i32>,
    pub cost_price: Decimal,
    pub selling_price: Decimal,
    #[validate(range(min = 0))]
    pub stock_quantity: i32,
    #[validate(range(min = 0))]
    pub low_stock_threshold: i32,
    pub tax_rate: Decimal,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateProductRequest {
    #[validate(length(max = 100))]
    pub barcode: Option<String>,
    #[validate(length(min = 1, max = 150))]
    pub name: Option<String>,
    pub description: Option<String>,
    pub category_id: Option<i32>,
    pub cost_price: Option<Decimal>,
    pub selling_price: Option<Decimal>,
    #[validate(range(min = 0))]
    pub stock_quantity: Option<i32>,
    #[validate(range(min = 0))]
    pub low_stock_threshold: Option<i32>,
    pub tax_rate: Option<Decimal>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct StockAdjustmentRequest {
    pub change_amount: i32,
    pub reason: String,
}

#[derive(Debug, Serialize)]
pub struct ProductResponse {
    pub id: i32,
    pub sku: String,
    pub barcode: Option<String>,
    pub name: String,
    pub description: Option<String>,
    pub category_id: Option<i32>,
    pub selling_price: Decimal,
    pub stock_quantity: i32,
    pub low_stock_threshold: i32,
    pub tax_rate: Decimal,
    pub is_active: bool,
    pub is_low_stock: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<Product> for ProductResponse {
    fn from(product: Product) -> Self {
        ProductResponse {
            is_low_stock: product.stock_quantity <= product.low_stock_threshold,
            id: product.id,
            sku: product.sku,
            barcode: product.barcode,
            name: product.name,
            description: product.description,
            category_id: product.category_id,
            selling_price: product.selling_price,
            stock_quantity: product.stock_quantity,
            low_stock_threshold: product.low_stock_threshold,
            tax_rate: product.tax_rate,
            is_active: product.is_active,
            created_at: product.created_at,
            updated_at: product.updated_at,
        }
    }
}
