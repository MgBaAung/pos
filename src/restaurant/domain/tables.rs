use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use validator::Validate;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "table_status", rename_all = "snake_case")]
pub enum TableStatus {
    Available,
    Occupied,
    Reserved,
    Cleaning,
    Maintenance,
}

impl TableStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            TableStatus::Available => "available",
            TableStatus::Occupied => "occupied",
            TableStatus::Reserved => "reserved",
            TableStatus::Cleaning => "cleaning",
            TableStatus::Maintenance => "maintenance",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct RestaurantTable {
    pub id: i32,
    pub table_number: String,
    pub capacity: i32,
    pub status: TableStatus,
    pub zone: Option<String>,
    pub location_description: Option<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateTableRequest {
    #[validate(length(min = 1, max = 20))]
    pub table_number: String,
    #[validate(range(min = 1))]
    pub capacity: i32,
    pub status: Option<TableStatus>,
    #[validate(length(max = 50))]
    pub zone: Option<String>,
    pub location_description: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateTableRequest {
    #[validate(range(min = 1))]
    pub capacity: Option<i32>,
    pub status: Option<TableStatus>,
    #[validate(length(max = 50))]
    pub zone: Option<String>,
    pub location_description: Option<String>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct TableResponse {
    pub id: i32,
    pub table_number: String,
    pub capacity: i32,
    pub status: TableStatus,
    pub zone: Option<String>,
    pub location_description: Option<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<RestaurantTable> for TableResponse {
    fn from(table: RestaurantTable) -> Self {
        TableResponse {
            id: table.id,
            table_number: table.table_number,
            capacity: table.capacity,
            status: table.status,
            zone: table.zone,
            location_description: table.location_description,
            is_active: table.is_active,
            created_at: table.created_at,
            updated_at: table.updated_at,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct AssignTableRequest {
    pub table_id: i32,
    pub guest_count: i32,
}
