use crate::restaurant::domain::kitchen_orders::OrderType;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::FromRow;
use validator::Validate;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "payment_method", rename_all = "snake_case")]
pub enum PaymentMethod {
    Cash,
    Card,
    MobileQr,
}

impl PaymentMethod {
    pub fn as_str(&self) -> &'static str {
        match self {
            PaymentMethod::Cash => "cash",
            PaymentMethod::Card => "card",
            PaymentMethod::MobileQr => "mobile_qr",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "sale_status", rename_all = "snake_case")]
pub enum SaleStatus {
    Completed,
    Refunded,
    Cancelled,
}

impl SaleStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            SaleStatus::Completed => "completed",
            SaleStatus::Refunded => "refunded",
            SaleStatus::Cancelled => "cancelled",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Sale {
    pub id: i32,
    pub invoice_no: String,
    pub cashier_id: Option<i32>,
    pub customer_id: Option<i32>,
    pub subtotal: Decimal,
    pub tax_total: Decimal,
    pub discount_total: Decimal,
    pub grand_total: Decimal,
    pub amount_paid: Decimal,
    pub change_given: Decimal,
    pub payment_method: PaymentMethod,
    pub status: SaleStatus,
    pub table_id: Option<i32>,
    pub order_type: OrderType,
    pub guest_count: Option<i32>,
    pub special_requests: Option<String>,
    pub loyalty_discount: Decimal,
    pub points_redeemed: i32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct SaleResponse {
    pub id: i32,
    pub invoice_no: String,
    pub cashier_id: Option<i32>,
    pub customer_id: Option<i32>,
    pub subtotal: Decimal,
    pub tax_total: Decimal,
    pub discount_total: Decimal,
    pub grand_total: Decimal,
    pub amount_paid: Decimal,
    pub change_given: Decimal,
    pub payment_method: PaymentMethod,
    pub status: SaleStatus,
    pub table_id: Option<i32>,
    pub order_type: OrderType,
    pub guest_count: Option<i32>,
    pub special_requests: Option<String>,
    pub loyalty_discount: Decimal,
    pub points_redeemed: i32,
    pub created_at: DateTime<Utc>,
    pub items: Vec<SaleItemResponse>,
}

impl SaleResponse {
    /// Build a response from a persisted sale and its line items.
    pub fn new(sale: Sale, items: Vec<SaleItem>) -> Self {
        SaleResponse {
            id: sale.id,
            invoice_no: sale.invoice_no,
            cashier_id: sale.cashier_id,
            customer_id: sale.customer_id,
            subtotal: sale.subtotal,
            tax_total: sale.tax_total,
            discount_total: sale.discount_total,
            grand_total: sale.grand_total,
            amount_paid: sale.amount_paid,
            change_given: sale.change_given,
            payment_method: sale.payment_method,
            status: sale.status,
            table_id: sale.table_id,
            order_type: sale.order_type,
            guest_count: sale.guest_count,
            special_requests: sale.special_requests,
            loyalty_discount: sale.loyalty_discount,
            points_redeemed: sale.points_redeemed,
            created_at: sale.created_at,
            items: items.into_iter().map(SaleItemResponse::from).collect(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SaleItem {
    pub id: i32,
    pub sale_id: i32,
    pub product_id: Option<i32>,
    pub quantity: i32,
    pub unit_price: Decimal,
    pub cost_price: Decimal,
    pub discount: Decimal,
    pub subtotal: Decimal,
    #[serde(default)]
    pub selected_modifiers: Value,
}

#[derive(Debug, Serialize)]
pub struct SaleItemResponse {
    pub id: i32,
    pub product_id: Option<i32>,
    pub quantity: i32,
    pub unit_price: Decimal,
    pub discount: Decimal,
    pub subtotal: Decimal,
    pub selected_modifiers: Value,
}

impl From<SaleItem> for SaleItemResponse {
    fn from(item: SaleItem) -> Self {
        SaleItemResponse {
            id: item.id,
            product_id: item.product_id,
            quantity: item.quantity,
            unit_price: item.unit_price,
            discount: item.discount,
            subtotal: item.subtotal,
            selected_modifiers: item.selected_modifiers,
        }
    }
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateSaleRequest {
    pub customer_id: Option<i32>,
    pub payment_method: PaymentMethod,
    pub amount_paid: Decimal,
    pub items: Vec<CreateSaleItemRequest>,
    pub discount_total: Option<Decimal>,
    // Dine-in / restaurant fields (all optional; a plain retail sale leaves them empty).
    pub table_id: Option<i32>,
    pub order_type: Option<OrderType>,
    #[validate(range(min = 1))]
    pub guest_count: Option<i32>,
    pub special_requests: Option<String>,
    /// Loyalty points to redeem against this sale (requires `customer_id`).
    #[validate(range(min = 0))]
    pub redeem_points: Option<i32>,
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct CreateSaleItemRequest {
    pub product_id: i32,
    #[validate(range(min = 1))]
    pub quantity: i32,
    pub discount: Option<Decimal>,
    /// Modifier ids chosen for this line; their price adjustments are added to the unit price.
    pub selected_modifiers: Option<Vec<i32>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CashDrawerSession {
    pub id: i32,
    pub user_id: Option<i32>,
    pub opening_cash: Decimal,
    pub closing_cash_expected: Decimal,
    pub closing_cash_actual: Decimal,
    pub discrepancy: Decimal,
    pub notes: Option<String>,
    pub opened_at: DateTime<Utc>,
    pub closed_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateCashDrawerSessionRequest {
    pub opening_cash: Decimal,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CloseCashDrawerSessionRequest {
    pub closing_cash_actual: Decimal,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CartItem {
    pub product_id: i32,
    pub quantity: i32,
    pub discount: Option<Decimal>,
}

#[derive(Debug, Serialize)]
pub struct ReceiptData {
    pub invoice_no: String,
    pub customer_name: Option<String>,
    pub cashier_name: String,
    pub items: Vec<ReceiptItem>,
    pub subtotal: Decimal,
    pub tax_total: Decimal,
    pub discount_total: Decimal,
    pub grand_total: Decimal,
    pub amount_paid: Decimal,
    pub change_given: Decimal,
    pub payment_method: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct ReceiptItem {
    pub name: String,
    pub quantity: i32,
    pub unit_price: Decimal,
    pub discount: Decimal,
    pub subtotal: Decimal,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;
    use serde_json::json;

    fn sale_item(modifiers: Value) -> SaleItem {
        SaleItem {
            id: 1,
            sale_id: 10,
            product_id: Some(5),
            quantity: 2,
            unit_price: dec!(12.50),
            cost_price: dec!(6.00),
            discount: dec!(0),
            subtotal: dec!(25.00),
            selected_modifiers: modifiers,
        }
    }

    fn sale() -> Sale {
        Sale {
            id: 10,
            invoice_no: "INV-TEST".into(),
            cashier_id: Some(1),
            customer_id: None,
            subtotal: dec!(25.00),
            tax_total: dec!(0),
            discount_total: dec!(0),
            grand_total: dec!(25.00),
            amount_paid: dec!(30.00),
            change_given: dec!(5.00),
            payment_method: PaymentMethod::Cash,
            status: SaleStatus::Completed,
            table_id: Some(7),
            order_type: OrderType::DineIn,
            guest_count: Some(4),
            special_requests: Some("no onions".into()),
            loyalty_discount: dec!(0),
            points_redeemed: 0,
            created_at: Utc::now(),
        }
    }

    #[test]
    fn sale_response_new_maps_dine_in_fields() {
        let resp = SaleResponse::new(sale(), vec![sale_item(json!([]))]);
        assert_eq!(resp.table_id, Some(7));
        assert_eq!(resp.order_type, OrderType::DineIn);
        assert_eq!(resp.guest_count, Some(4));
        assert_eq!(resp.special_requests.as_deref(), Some("no onions"));
        assert_eq!(resp.items.len(), 1);
    }

    #[test]
    fn sale_item_response_carries_selected_modifiers() {
        let modifiers = json!([{ "id": 3, "name": "Extra Cheese", "price_adjustment": "2.50" }]);
        let resp = SaleItemResponse::from(sale_item(modifiers.clone()));
        assert_eq!(resp.selected_modifiers, modifiers);
    }
}
