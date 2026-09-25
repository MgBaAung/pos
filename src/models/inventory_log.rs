use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "adjustment_type", rename_all = "snake_case")]
pub enum AdjustmentType {
    Restock,
    Damage,
    ManualCorrection,
    Sale,
}

impl AdjustmentType {
    pub fn as_str(&self) -> &'static str {
        match self {
            AdjustmentType::Restock => "restock",
            AdjustmentType::Damage => "damage",
            AdjustmentType::ManualCorrection => "manual_correction",
            AdjustmentType::Sale => "sale",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct InventoryLog {
    pub id: i32,
    pub product_id: i32,
    pub user_id: Option<i32>,
    pub change_amount: i32,
    #[serde(rename = "type")]
    pub adjustment_type: AdjustmentType,
    pub reason: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateInventoryLogRequest {
    pub product_id: i32,
    pub change_amount: i32,
    pub adjustment_type: AdjustmentType,
    pub reason: String,
}
