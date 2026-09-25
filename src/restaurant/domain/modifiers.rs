use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ModifierGroup {
    pub id: i32,
    pub name: String,
    pub description: Option<String>,
    pub is_required: bool,
    pub min_selections: i32,
    pub max_selections: i32,
    pub display_order: i32,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateModifierGroupRequest {
    #[validate(length(min = 1, max = 100))]
    pub name: String,
    pub description: Option<String>,
    pub is_required: Option<bool>,
    #[validate(range(min = 0))]
    pub min_selections: Option<i32>,
    #[validate(range(min = 1))]
    pub max_selections: Option<i32>,
    pub display_order: Option<i32>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateModifierGroupRequest {
    #[validate(length(max = 100))]
    pub name: Option<String>,
    pub description: Option<String>,
    pub is_required: Option<bool>,
    #[validate(range(min = 0))]
    pub min_selections: Option<i32>,
    #[validate(range(min = 1))]
    pub max_selections: Option<i32>,
    pub display_order: Option<i32>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Modifier {
    pub id: i32,
    pub group_id: i32,
    pub name: String,
    pub description: Option<String>,
    pub price_adjustment: Decimal,
    pub is_available: bool,
    pub display_order: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateModifierRequest {
    pub group_id: i32,
    #[validate(length(min = 1, max = 100))]
    pub name: String,
    pub description: Option<String>,
    pub price_adjustment: Option<Decimal>,
    pub display_order: Option<i32>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateModifierRequest {
    #[validate(length(max = 100))]
    pub name: Option<String>,
    pub description: Option<String>,
    pub price_adjustment: Option<Decimal>,
    pub is_available: Option<bool>,
    pub display_order: Option<i32>,
}

#[derive(Debug, Serialize)]
pub struct ModifierGroupResponse {
    pub id: i32,
    pub name: String,
    pub description: Option<String>,
    pub is_required: bool,
    pub min_selections: i32,
    pub max_selections: i32,
    pub display_order: i32,
    pub is_active: bool,
    pub modifiers: Vec<ModifierResponse>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct ModifierResponse {
    pub id: i32,
    pub group_id: i32,
    pub name: String,
    pub description: Option<String>,
    pub price_adjustment: Decimal,
    pub is_available: bool,
    pub display_order: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct LinkProductToModifierGroupRequest {
    pub product_id: i32,
    pub modifier_group_id: i32,
    pub is_required: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SelectedModifier {
    pub modifier_id: i32,
    pub quantity: i32,
}
