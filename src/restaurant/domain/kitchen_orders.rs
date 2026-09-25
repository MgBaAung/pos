use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use validator::Validate;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "kot_status", rename_all = "snake_case")]
pub enum KotStatus {
    Pending,
    Preparing,
    Ready,
    Served,
    Cancelled,
    Delayed,
}

impl KotStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            KotStatus::Pending => "pending",
            KotStatus::Preparing => "preparing",
            KotStatus::Ready => "ready",
            KotStatus::Served => "served",
            KotStatus::Cancelled => "cancelled",
            KotStatus::Delayed => "delayed",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "order_type", rename_all = "snake_case")]
pub enum OrderType {
    DineIn,
    Takeaway,
    Delivery,
    Retail,
}

impl OrderType {
    pub fn as_str(&self) -> &'static str {
        match self {
            OrderType::DineIn => "dine_in",
            OrderType::Takeaway => "takeaway",
            OrderType::Delivery => "delivery",
            OrderType::Retail => "retail",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct KitchenOrder {
    pub id: i32,
    pub sale_id: i32,
    pub table_id: Option<i32>,
    pub order_type: OrderType,
    pub status: KotStatus,
    pub priority: i32,
    pub notes: Option<String>,
    pub estimated_preparation_time: Option<i32>,
    pub actual_preparation_time: Option<i32>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateKitchenOrderRequest {
    pub sale_id: i32,
    pub table_id: Option<i32>,
    pub order_type: OrderType,
    pub priority: Option<i32>,
    pub notes: Option<String>,
    pub estimated_preparation_time: Option<i32>,
    pub items: Vec<CreateKitchenOrderItemRequest>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateKitchenOrderRequest {
    pub status: Option<KotStatus>,
    pub priority: Option<i32>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct KitchenOrderItem {
    pub id: i32,
    pub kitchen_order_id: i32,
    pub product_id: Option<i32>,
    pub product_name: String,
    pub quantity: i32,
    pub unit_price: Decimal,
    pub selected_modifiers: serde_json::Value,
    pub special_instructions: Option<String>,
    pub item_status: KotStatus,
    pub sequence_order: i32,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateKitchenOrderItemRequest {
    pub product_id: i32,
    #[validate(range(min = 1))]
    pub quantity: i32,
    pub special_instructions: Option<String>,
    pub selected_modifiers: Vec<super::modifiers::SelectedModifier>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateKitchenOrderItemRequest {
    pub item_status: Option<KotStatus>,
}

#[derive(Debug, Serialize)]
pub struct KitchenOrderResponse {
    pub id: i32,
    pub sale_id: i32,
    pub table_id: Option<i32>,
    pub table_number: Option<String>,
    pub order_type: OrderType,
    pub status: KotStatus,
    pub priority: i32,
    pub notes: Option<String>,
    pub estimated_preparation_time: Option<i32>,
    pub actual_preparation_time: Option<i32>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub items: Vec<KitchenOrderItemResponse>,
}

#[derive(Debug, Serialize)]
pub struct KitchenOrderItemResponse {
    pub id: i32,
    pub kitchen_order_id: i32,
    pub product_id: Option<i32>,
    pub product_name: String,
    pub quantity: i32,
    pub unit_price: Decimal,
    pub selected_modifiers: serde_json::Value,
    pub special_instructions: Option<String>,
    pub item_status: KotStatus,
    pub sequence_order: i32,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct KitchenOrderEvent {
    pub event_type: String,
    pub kitchen_order_id: i32,
    pub status: KotStatus,
    pub table_id: Option<i32>,
    pub table_number: Option<String>,
    pub timestamp: DateTime<Utc>,
}
