use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use validator::Validate;

/// A concrete, purchasable option of a base product (e.g. "Red / Large").
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ProductVariant {
    pub id: i32,
    pub product_id: i32,
    pub sku: String,
    pub barcode: Option<String>,
    pub name: String,
    pub size: String,
    pub color: String,
    pub cost_price: Decimal,
    pub selling_price: Decimal,
    pub stock_quantity: i32,
    pub low_stock_threshold: i32,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateProductVariantRequest {
    #[validate(length(min = 1, max = 100))]
    pub sku: String,
    #[validate(length(max = 100))]
    pub barcode: Option<String>,
    #[validate(length(min = 1, max = 150))]
    pub name: String,
    #[validate(length(max = 50))]
    pub size: Option<String>,
    #[validate(length(max = 50))]
    pub color: Option<String>,
    pub cost_price: Decimal,
    pub selling_price: Decimal,
    #[validate(range(min = 0))]
    pub stock_quantity: i32,
    #[validate(range(min = 0))]
    pub low_stock_threshold: i32,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateProductVariantRequest {
    #[validate(length(max = 100))]
    pub barcode: Option<String>,
    #[validate(length(min = 1, max = 150))]
    pub name: Option<String>,
    #[validate(length(max = 50))]
    pub size: Option<String>,
    #[validate(length(max = 50))]
    pub color: Option<String>,
    pub cost_price: Option<Decimal>,
    pub selling_price: Option<Decimal>,
    #[validate(range(min = 0))]
    pub stock_quantity: Option<i32>,
    #[validate(range(min = 0))]
    pub low_stock_threshold: Option<i32>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductVariantResponse {
    pub id: i32,
    pub product_id: i32,
    pub sku: String,
    pub barcode: Option<String>,
    pub name: String,
    pub size: String,
    pub color: String,
    pub cost_price: Decimal,
    pub selling_price: Decimal,
    pub stock_quantity: i32,
    pub low_stock_threshold: i32,
    pub is_low_stock: bool,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<ProductVariant> for ProductVariantResponse {
    fn from(variant: ProductVariant) -> Self {
        ProductVariantResponse {
            is_low_stock: variant.stock_quantity <= variant.low_stock_threshold,
            id: variant.id,
            product_id: variant.product_id,
            sku: variant.sku,
            barcode: variant.barcode,
            name: variant.name,
            size: variant.size,
            color: variant.color,
            cost_price: variant.cost_price,
            selling_price: variant.selling_price,
            stock_quantity: variant.stock_quantity,
            low_stock_threshold: variant.low_stock_threshold,
            is_active: variant.is_active,
            created_at: variant.created_at,
            updated_at: variant.updated_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;
    use validator::Validate;

    fn variant(stock: i32, threshold: i32) -> ProductVariant {
        ProductVariant {
            id: 1,
            product_id: 2,
            sku: "V-1".into(),
            barcode: None,
            name: "Red / L".into(),
            size: "L".into(),
            color: "Red".into(),
            cost_price: dec!(5.00),
            selling_price: dec!(10.00),
            stock_quantity: stock,
            low_stock_threshold: threshold,
            is_active: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn create_request(sku: &str, name: &str, stock: i32) -> CreateProductVariantRequest {
        CreateProductVariantRequest {
            sku: sku.into(),
            barcode: None,
            name: name.into(),
            size: Some("M".into()),
            color: Some("Blue".into()),
            cost_price: dec!(1.00),
            selling_price: dec!(2.00),
            stock_quantity: stock,
            low_stock_threshold: 0,
        }
    }

    #[test]
    fn response_flags_low_stock_at_or_below_threshold() {
        assert!(ProductVariantResponse::from(variant(5, 5)).is_low_stock);
        assert!(ProductVariantResponse::from(variant(0, 5)).is_low_stock);
        assert!(!ProductVariantResponse::from(variant(6, 5)).is_low_stock);
    }

    #[test]
    fn response_copies_core_fields() {
        let r = ProductVariantResponse::from(variant(10, 5));
        assert_eq!(r.sku, "V-1");
        assert_eq!(r.size, "L");
        assert_eq!(r.color, "Red");
        assert_eq!(r.selling_price, dec!(10.00));
        assert!(r.is_active);
    }

    #[test]
    fn create_request_rejects_empty_sku_and_name() {
        assert!(create_request("", "", 1).validate().is_err());
    }

    #[test]
    fn create_request_rejects_negative_stock() {
        assert!(create_request("V-2", "ok", -1).validate().is_err());
    }

    #[test]
    fn create_request_accepts_valid_input() {
        assert!(create_request("V-3", "Blue / M", 10).validate().is_ok());
    }

    #[test]
    fn update_request_with_no_fields_is_valid() {
        let empty = UpdateProductVariantRequest {
            barcode: None,
            name: None,
            size: None,
            color: None,
            cost_price: None,
            selling_price: None,
            stock_quantity: None,
            low_stock_threshold: None,
            is_active: None,
        };
        assert!(empty.validate().is_ok());
    }
}
