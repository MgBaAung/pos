use crate::database::Database;
use crate::error::AppError;
use crate::restaurant::domain::tables::{
    CreateTableRequest, RestaurantTable, TableResponse, TableStatus, UpdateTableRequest,
};
use sqlx::PgPool;

pub struct TableService {
    pool: PgPool,
}

impl TableService {
    pub fn new(database: &Database) -> Self {
        Self {
            pool: database.pool.clone(),
        }
    }

    pub async fn create_table(
        &self,
        request: CreateTableRequest,
    ) -> Result<TableResponse, AppError> {
        let status = request.status.unwrap_or(TableStatus::Available);

        let table = sqlx::query_as::<_, RestaurantTable>(
            r#"
            INSERT INTO restaurant_tables (table_number, capacity, status, zone, location_description)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING *
            "#
        )
        .bind(&request.table_number)
        .bind(request.capacity)
        .bind(status)
        .bind(&request.zone)
        .bind(&request.location_description)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            if e.to_string().contains("restaurant_tables_table_number_key") {
                AppError::Conflict("Table number already exists".to_string())
            } else {
                AppError::Database(e)
            }
        })?;

        Ok(TableResponse::from(table))
    }

    pub async fn get_table(&self, id: i32) -> Result<TableResponse, AppError> {
        let table =
            sqlx::query_as::<_, RestaurantTable>("SELECT * FROM restaurant_tables WHERE id = $1")
                .bind(id)
                .fetch_one(&self.pool)
                .await
                .map_err(|e| {
                    if e.to_string().contains("no rows returned") {
                        AppError::NotFound("Table not found".to_string())
                    } else {
                        AppError::Database(e)
                    }
                })?;

        Ok(TableResponse::from(table))
    }

    pub async fn get_table_by_number(&self, table_number: &str) -> Result<TableResponse, AppError> {
        let table = sqlx::query_as::<_, RestaurantTable>(
            "SELECT * FROM restaurant_tables WHERE table_number = $1",
        )
        .bind(table_number)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            if e.to_string().contains("no rows returned") {
                AppError::NotFound("Table not found".to_string())
            } else {
                AppError::Database(e)
            }
        })?;

        Ok(TableResponse::from(table))
    }

    pub async fn list_tables(
        &self,
        status: Option<TableStatus>,
        zone: Option<String>,
        is_active: Option<bool>,
    ) -> Result<Vec<TableResponse>, AppError> {
        let mut query = "SELECT * FROM restaurant_tables WHERE 1=1".to_string();
        let mut conditions = Vec::new();
        let mut param_count = 0;

        if status.is_some() {
            param_count += 1;
            conditions.push(format!("AND status = ${}", param_count));
        }
        if zone.is_some() {
            param_count += 1;
            conditions.push(format!("AND zone = ${}", param_count));
        }
        if is_active.is_some() {
            param_count += 1;
            conditions.push(format!("AND is_active = ${}", param_count));
        }

        query.push_str(&conditions.join(" "));
        query.push_str(" ORDER BY table_number");

        let mut query_builder = sqlx::query_as::<_, RestaurantTable>(&query);

        if let Some(status) = status {
            query_builder = query_builder.bind(status);
        }
        if let Some(zone) = zone {
            query_builder = query_builder.bind(zone);
        }
        if let Some(is_active) = is_active {
            query_builder = query_builder.bind(is_active);
        }

        let tables = query_builder.fetch_all(&self.pool).await?;

        Ok(tables.into_iter().map(TableResponse::from).collect())
    }

    pub async fn update_table(
        &self,
        id: i32,
        request: UpdateTableRequest,
    ) -> Result<TableResponse, AppError> {
        let mut updates = Vec::new();
        let mut param_count = 0;

        if request.capacity.is_some() {
            param_count += 1;
            updates.push(format!("capacity = ${}", param_count));
        }
        if request.status.is_some() {
            param_count += 1;
            updates.push(format!("status = ${}", param_count));
        }
        if request.zone.is_some() {
            param_count += 1;
            updates.push(format!("zone = ${}", param_count));
        }
        if request.location_description.is_some() {
            param_count += 1;
            updates.push(format!("location_description = ${}", param_count));
        }
        if request.is_active.is_some() {
            param_count += 1;
            updates.push(format!("is_active = ${}", param_count));
        }

        if updates.is_empty() {
            return self.get_table(id).await;
        }

        let query = format!(
            "UPDATE restaurant_tables SET {} WHERE id = ${} RETURNING *",
            updates.join(", "),
            param_count + 1
        );

        let mut query_builder = sqlx::query_as::<_, RestaurantTable>(&query);

        if let Some(capacity) = request.capacity {
            query_builder = query_builder.bind(capacity);
        }
        if let Some(status) = request.status {
            query_builder = query_builder.bind(status);
        }
        if let Some(zone) = request.zone {
            query_builder = query_builder.bind(zone);
        }
        if let Some(location_description) = request.location_description {
            query_builder = query_builder.bind(location_description);
        }
        if let Some(is_active) = request.is_active {
            query_builder = query_builder.bind(is_active);
        }
        query_builder = query_builder.bind(id);

        let table = query_builder.fetch_one(&self.pool).await.map_err(|e| {
            if e.to_string().contains("no rows returned") {
                AppError::NotFound("Table not found".to_string())
            } else {
                AppError::Database(e)
            }
        })?;

        Ok(TableResponse::from(table))
    }

    pub async fn delete_table(&self, id: i32) -> Result<(), AppError> {
        let result = sqlx::query("DELETE FROM restaurant_tables WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound("Table not found".to_string()));
        }

        Ok(())
    }

    pub async fn get_available_tables(
        &self,
        guest_count: Option<i32>,
    ) -> Result<Vec<TableResponse>, AppError> {
        let query = if let Some(guest_count) = guest_count {
            sqlx::query_as::<_, RestaurantTable>(
                "SELECT * FROM restaurant_tables WHERE status = $1 AND capacity >= $2 AND is_active = TRUE ORDER BY capacity ASC"
            )
            .bind(TableStatus::Available)
            .bind(guest_count)
        } else {
            sqlx::query_as::<_, RestaurantTable>(
                "SELECT * FROM restaurant_tables WHERE status = $1 AND is_active = TRUE ORDER BY table_number"
            )
            .bind(TableStatus::Available)
        };

        let tables = query.fetch_all(&self.pool).await?;
        Ok(tables.into_iter().map(TableResponse::from).collect())
    }

    pub async fn update_table_status(
        &self,
        id: i32,
        status: TableStatus,
    ) -> Result<TableResponse, AppError> {
        let table = sqlx::query_as::<_, RestaurantTable>(
            "UPDATE restaurant_tables SET status = $1 WHERE id = $2 RETURNING *",
        )
        .bind(status)
        .bind(id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            if e.to_string().contains("no rows returned") {
                AppError::NotFound("Table not found".to_string())
            } else {
                AppError::Database(e)
            }
        })?;

        Ok(TableResponse::from(table))
    }
}
