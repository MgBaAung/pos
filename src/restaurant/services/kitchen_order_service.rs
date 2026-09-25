use crate::database::Database;
use crate::error::AppError;
use crate::restaurant::domain::kitchen_orders::{
    CreateKitchenOrderRequest, KitchenOrder, KitchenOrderEvent, KitchenOrderItem,
    KitchenOrderItemResponse, KitchenOrderResponse, KotStatus, UpdateKitchenOrderItemRequest,
    UpdateKitchenOrderRequest,
};
use sqlx::PgPool;
use tokio::sync::broadcast;

pub struct KitchenOrderService {
    pool: PgPool,
    kitchen_event_sender: broadcast::Sender<KitchenOrderEvent>,
}

impl KitchenOrderService {
    pub fn new(
        database: &Database,
        kitchen_event_sender: broadcast::Sender<KitchenOrderEvent>,
    ) -> Self {
        Self {
            pool: database.pool.clone(),
            kitchen_event_sender,
        }
    }

    pub async fn create_kitchen_order(
        &self,
        request: CreateKitchenOrderRequest,
    ) -> Result<KitchenOrderResponse, AppError> {
        let mut tx = self.pool.begin().await?;

        // Get table number if table_id is provided
        let table_number = if let Some(table_id) = request.table_id {
            let table: Option<(String,)> =
                sqlx::query_as("SELECT table_number FROM restaurant_tables WHERE id = $1")
                    .bind(table_id)
                    .fetch_optional(&mut *tx)
                    .await?;
            table.map(|t| t.0)
        } else {
            None
        };

        // Create kitchen order
        let kitchen_order = sqlx::query_as::<_, KitchenOrder>(
            r#"
            INSERT INTO kitchen_orders (sale_id, table_id, order_type, status, priority, notes, estimated_preparation_time)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING *
            "#
        )
        .bind(request.sale_id)
        .bind(request.table_id)
        .bind(request.order_type)
        .bind(KotStatus::Pending)
        .bind(request.priority.unwrap_or(5))
        .bind(&request.notes)
        .bind(request.estimated_preparation_time)
        .fetch_one(&mut *tx)
        .await?;

        // Create kitchen order items
        let mut kitchen_order_items = Vec::new();
        for (index, item_request) in request.items.iter().enumerate() {
            // Get product name
            let product: (String, rust_decimal::Decimal) =
                sqlx::query_as("SELECT name, selling_price FROM products WHERE id = $1")
                    .bind(item_request.product_id)
                    .fetch_one(&mut *tx)
                    .await?;

            let selected_modifiers_json = serde_json::to_value(&item_request.selected_modifiers)
                .map_err(|e| AppError::Internal(format!("Failed to serialize modifiers: {}", e)))?;

            let item = sqlx::query_as::<_, KitchenOrderItem>(
                r#"
                INSERT INTO kitchen_order_items 
                (kitchen_order_id, product_id, product_name, quantity, unit_price, selected_modifiers, special_instructions, sequence_order)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                RETURNING *
                "#
            )
            .bind(kitchen_order.id)
            .bind(item_request.product_id)
            .bind(&product.0)
            .bind(item_request.quantity)
            .bind(product.1)
            .bind(&selected_modifiers_json)
            .bind(&item_request.special_instructions)
            .bind(index as i32)
            .fetch_one(&mut *tx)
            .await?;

            kitchen_order_items.push(item);
        }

        tx.commit().await?;

        // Broadcast event
        let event = KitchenOrderEvent {
            event_type: "kitchen_order_created".to_string(),
            kitchen_order_id: kitchen_order.id,
            status: kitchen_order.status,
            table_id: kitchen_order.table_id,
            table_number: table_number.clone(),
            timestamp: chrono::Utc::now(),
        };
        let _ = self.kitchen_event_sender.send(event);

        Ok(self
            .build_kitchen_order_response(kitchen_order, table_number, kitchen_order_items)
            .await)
    }

    pub async fn get_kitchen_order(&self, id: i32) -> Result<KitchenOrderResponse, AppError> {
        let kitchen_order =
            sqlx::query_as::<_, KitchenOrder>("SELECT * FROM kitchen_orders WHERE id = $1")
                .bind(id)
                .fetch_one(&self.pool)
                .await
                .map_err(|e| {
                    if e.to_string().contains("no rows returned") {
                        AppError::NotFound("Kitchen order not found".to_string())
                    } else {
                        AppError::Database(e)
                    }
                })?;

        let table_number = if let Some(table_id) = kitchen_order.table_id {
            let table: Option<(String,)> =
                sqlx::query_as("SELECT table_number FROM restaurant_tables WHERE id = $1")
                    .bind(table_id)
                    .fetch_optional(&self.pool)
                    .await?;
            table.map(|(t,)| t)
        } else {
            None
        };

        let items = self.get_kitchen_order_items(id).await?;

        Ok(KitchenOrderResponse {
            id: kitchen_order.id,
            sale_id: kitchen_order.sale_id,
            table_id: kitchen_order.table_id,
            table_number,
            order_type: kitchen_order.order_type,
            status: kitchen_order.status,
            priority: kitchen_order.priority,
            notes: kitchen_order.notes,
            estimated_preparation_time: kitchen_order.estimated_preparation_time,
            actual_preparation_time: kitchen_order.actual_preparation_time,
            started_at: kitchen_order.started_at,
            completed_at: kitchen_order.completed_at,
            created_at: kitchen_order.created_at,
            updated_at: kitchen_order.updated_at,
            items,
        })
    }

    pub async fn list_kitchen_orders(
        &self,
        status: Option<KotStatus>,
        table_id: Option<i32>,
    ) -> Result<Vec<KitchenOrderResponse>, AppError> {
        let mut query = "SELECT * FROM kitchen_orders WHERE 1=1".to_string();
        let mut conditions = Vec::new();
        let mut param_count = 0;

        if status.is_some() {
            param_count += 1;
            conditions.push(format!("AND status = ${}", param_count));
        }
        if table_id.is_some() {
            param_count += 1;
            conditions.push(format!("AND table_id = ${}", param_count));
        }

        query.push_str(&conditions.join(" "));
        query.push_str(" ORDER BY priority ASC, created_at ASC");

        let mut query_builder = sqlx::query_as::<_, KitchenOrder>(&query);

        if let Some(status) = status {
            query_builder = query_builder.bind(status);
        }
        if let Some(table_id) = table_id {
            query_builder = query_builder.bind(table_id);
        }

        let kitchen_orders = query_builder.fetch_all(&self.pool).await?;

        let mut responses = Vec::new();
        for ko in kitchen_orders {
            let table_number = if let Some(table_id) = ko.table_id {
                let table: Option<(String,)> =
                    sqlx::query_as("SELECT table_number FROM restaurant_tables WHERE id = $1")
                        .bind(table_id)
                        .fetch_optional(&self.pool)
                        .await?;
                table.map(|t| t.0)
            } else {
                None
            };

            let items = self.get_kitchen_order_items(ko.id).await?;

            responses.push(KitchenOrderResponse {
                id: ko.id,
                sale_id: ko.sale_id,
                table_id: ko.table_id,
                table_number,
                order_type: ko.order_type,
                status: ko.status,
                priority: ko.priority,
                notes: ko.notes,
                estimated_preparation_time: ko.estimated_preparation_time,
                actual_preparation_time: ko.actual_preparation_time,
                started_at: ko.started_at,
                completed_at: ko.completed_at,
                created_at: ko.created_at,
                updated_at: ko.updated_at,
                items,
            });
        }

        Ok(responses)
    }

    pub async fn update_kitchen_order(
        &self,
        id: i32,
        request: UpdateKitchenOrderRequest,
    ) -> Result<KitchenOrderResponse, AppError> {
        let mut updates = Vec::new();
        let mut param_count = 0;

        if request.status.is_some() {
            param_count += 1;
            updates.push(format!("status = ${}", param_count));
        }
        if request.priority.is_some() {
            param_count += 1;
            updates.push(format!("priority = ${}", param_count));
        }
        if request.notes.is_some() {
            param_count += 1;
            updates.push(format!("notes = ${}", param_count));
        }

        if updates.is_empty() {
            return self.get_kitchen_order(id).await;
        }

        let query = format!(
            "UPDATE kitchen_orders SET {} WHERE id = ${} RETURNING *",
            updates.join(", "),
            param_count + 1
        );

        let mut query_builder = sqlx::query_as::<_, KitchenOrder>(&query);

        if let Some(status) = request.status {
            query_builder = query_builder.bind(status);
        }
        if let Some(priority) = request.priority {
            query_builder = query_builder.bind(priority);
        }
        if let Some(notes) = request.notes {
            query_builder = query_builder.bind(notes);
        }
        query_builder = query_builder.bind(id);

        let kitchen_order = query_builder.fetch_one(&self.pool).await.map_err(|e| {
            if e.to_string().contains("no rows returned") {
                AppError::NotFound("Kitchen order not found".to_string())
            } else {
                AppError::Database(e)
            }
        })?;

        // Update timestamps based on status
        if let Some(status) = request.status {
            let _ = sqlx::query(
                r#"
                UPDATE kitchen_orders 
                SET started_at = CASE WHEN $1 = 'preparing' THEN CURRENT_TIMESTAMP ELSE started_at END,
                    completed_at = CASE WHEN $1 = 'ready' OR $1 = 'served' THEN CURRENT_TIMESTAMP ELSE completed_at END
                WHERE id = $2
                "#
            )
            .bind(status)
            .bind(id)
            .execute(&self.pool)
            .await;
        }

        let table_number = if let Some(table_id) = kitchen_order.table_id {
            let table: Option<(String,)> =
                sqlx::query_as("SELECT table_number FROM restaurant_tables WHERE id = $1")
                    .bind(table_id)
                    .fetch_optional(&self.pool)
                    .await?;
            table.map(|(t,)| t)
        } else {
            None
        };

        // Broadcast event
        let event = KitchenOrderEvent {
            event_type: "kitchen_order_updated".to_string(),
            kitchen_order_id: kitchen_order.id,
            status: kitchen_order.status,
            table_id: kitchen_order.table_id,
            table_number: table_number.clone(),
            timestamp: chrono::Utc::now(),
        };
        let _ = self.kitchen_event_sender.send(event);

        let items = self.get_kitchen_order_items(id).await?;

        Ok(KitchenOrderResponse {
            id: kitchen_order.id,
            sale_id: kitchen_order.sale_id,
            table_id: kitchen_order.table_id,
            table_number,
            order_type: kitchen_order.order_type,
            status: kitchen_order.status,
            priority: kitchen_order.priority,
            notes: kitchen_order.notes,
            estimated_preparation_time: kitchen_order.estimated_preparation_time,
            actual_preparation_time: kitchen_order.actual_preparation_time,
            started_at: kitchen_order.started_at,
            completed_at: kitchen_order.completed_at,
            created_at: kitchen_order.created_at,
            updated_at: kitchen_order.updated_at,
            items,
        })
    }

    pub async fn update_kitchen_order_item(
        &self,
        id: i32,
        request: UpdateKitchenOrderItemRequest,
    ) -> Result<KitchenOrderItemResponse, AppError> {
        if let Some(item_status) = request.item_status {
            let item = sqlx::query_as::<_, KitchenOrderItem>(
                r#"
                UPDATE kitchen_order_items 
                SET item_status = $1,
                    started_at = CASE WHEN $1 = 'preparing' THEN CURRENT_TIMESTAMP ELSE started_at END,
                    completed_at = CASE WHEN $1 = 'ready' OR $1 = 'served' THEN CURRENT_TIMESTAMP ELSE completed_at END
                WHERE id = $2
                RETURNING *
                "#
            )
            .bind(item_status)
            .bind(id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| {
                if e.to_string().contains("no rows returned") {
                    AppError::NotFound("Kitchen order item not found".to_string())
                } else {
                    AppError::Database(e)
                }
            })?;

            // Broadcast event for the parent kitchen order
            let event = KitchenOrderEvent {
                event_type: "kitchen_order_item_updated".to_string(),
                kitchen_order_id: item.kitchen_order_id,
                status: item.item_status,
                table_id: None,
                table_number: None,
                timestamp: chrono::Utc::now(),
            };
            let _ = self.kitchen_event_sender.send(event);

            Ok(KitchenOrderItemResponse {
                id: item.id,
                kitchen_order_id: item.kitchen_order_id,
                product_id: item.product_id,
                product_name: item.product_name,
                quantity: item.quantity,
                unit_price: item.unit_price,
                selected_modifiers: item.selected_modifiers,
                special_instructions: item.special_instructions,
                item_status: item.item_status,
                sequence_order: item.sequence_order,
                started_at: item.started_at,
                completed_at: item.completed_at,
                created_at: item.created_at,
                updated_at: item.updated_at,
            })
        } else {
            Err(AppError::BadRequest("No fields to update".to_string()))
        }
    }

    pub async fn get_pending_kitchen_orders(&self) -> Result<Vec<KitchenOrderResponse>, AppError> {
        self.list_kitchen_orders(Some(KotStatus::Pending), None)
            .await
    }

    pub async fn get_active_kitchen_orders(&self) -> Result<Vec<KitchenOrderResponse>, AppError> {
        let query = r#"
            SELECT * FROM kitchen_orders 
            WHERE status IN ('pending', 'preparing') 
            ORDER BY priority ASC, created_at ASC
        "#;

        let kitchen_orders = sqlx::query_as::<_, KitchenOrder>(query)
            .fetch_all(&self.pool)
            .await?;

        let mut responses = Vec::new();
        for ko in kitchen_orders {
            let table_number = if let Some(table_id) = ko.table_id {
                let table: Option<(String,)> =
                    sqlx::query_as("SELECT table_number FROM restaurant_tables WHERE id = $1")
                        .bind(table_id)
                        .fetch_optional(&self.pool)
                        .await?;
                table.map(|t| t.0)
            } else {
                None
            };

            let items = self.get_kitchen_order_items(ko.id).await?;

            responses.push(KitchenOrderResponse {
                id: ko.id,
                sale_id: ko.sale_id,
                table_id: ko.table_id,
                table_number,
                order_type: ko.order_type,
                status: ko.status,
                priority: ko.priority,
                notes: ko.notes,
                estimated_preparation_time: ko.estimated_preparation_time,
                actual_preparation_time: ko.actual_preparation_time,
                started_at: ko.started_at,
                completed_at: ko.completed_at,
                created_at: ko.created_at,
                updated_at: ko.updated_at,
                items,
            });
        }

        Ok(responses)
    }

    // Helper methods
    async fn get_kitchen_order_items(
        &self,
        kitchen_order_id: i32,
    ) -> Result<Vec<KitchenOrderItemResponse>, AppError> {
        let items = sqlx::query_as::<_, KitchenOrderItem>(
            "SELECT * FROM kitchen_order_items WHERE kitchen_order_id = $1 ORDER BY sequence_order",
        )
        .bind(kitchen_order_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(items
            .into_iter()
            .map(|item| KitchenOrderItemResponse {
                id: item.id,
                kitchen_order_id: item.kitchen_order_id,
                product_id: item.product_id,
                product_name: item.product_name,
                quantity: item.quantity,
                unit_price: item.unit_price,
                selected_modifiers: item.selected_modifiers,
                special_instructions: item.special_instructions,
                item_status: item.item_status,
                sequence_order: item.sequence_order,
                started_at: item.started_at,
                completed_at: item.completed_at,
                created_at: item.created_at,
                updated_at: item.updated_at,
            })
            .collect())
    }

    async fn build_kitchen_order_response(
        &self,
        kitchen_order: KitchenOrder,
        table_number: Option<String>,
        items: Vec<KitchenOrderItem>,
    ) -> KitchenOrderResponse {
        KitchenOrderResponse {
            id: kitchen_order.id,
            sale_id: kitchen_order.sale_id,
            table_id: kitchen_order.table_id,
            table_number,
            order_type: kitchen_order.order_type,
            status: kitchen_order.status,
            priority: kitchen_order.priority,
            notes: kitchen_order.notes,
            estimated_preparation_time: kitchen_order.estimated_preparation_time,
            actual_preparation_time: kitchen_order.actual_preparation_time,
            started_at: kitchen_order.started_at,
            completed_at: kitchen_order.completed_at,
            created_at: kitchen_order.created_at,
            updated_at: kitchen_order.updated_at,
            items: items
                .into_iter()
                .map(|item| KitchenOrderItemResponse {
                    id: item.id,
                    kitchen_order_id: item.kitchen_order_id,
                    product_id: item.product_id,
                    product_name: item.product_name,
                    quantity: item.quantity,
                    unit_price: item.unit_price,
                    selected_modifiers: item.selected_modifiers,
                    special_instructions: item.special_instructions,
                    item_status: item.item_status,
                    sequence_order: item.sequence_order,
                    started_at: item.started_at,
                    completed_at: item.completed_at,
                    created_at: item.created_at,
                    updated_at: item.updated_at,
                })
                .collect(),
        }
    }
}
