use crate::database::Database;
use crate::error::{AppError, AppResult};
use crate::models::{CreateCustomerRequest, Customer, UpdateCustomerRequest};
use rust_decimal::Decimal;

pub struct CustomerService {
    db: Database,
}

impl CustomerService {
    pub fn new(db: Database) -> Self {
        CustomerService { db }
    }

    /// Create a new customer
    pub async fn create_customer(&self, request: CreateCustomerRequest) -> AppResult<Customer> {
        // Validate phone uniqueness if provided
        if let Some(ref phone) = request.phone {
            let existing =
                sqlx::query_scalar::<_, i32>("SELECT id FROM customers WHERE phone = $1")
                    .bind(phone)
                    .fetch_optional(&self.db.pool)
                    .await?;

            if existing.is_some() {
                return Err(AppError::Conflict(format!(
                    "Phone number {} already registered",
                    phone
                )));
            }
        }

        // Validate email uniqueness if provided
        if let Some(ref email) = request.email {
            let existing =
                sqlx::query_scalar::<_, i32>("SELECT id FROM customers WHERE email = $1")
                    .bind(email)
                    .fetch_optional(&self.db.pool)
                    .await?;

            if existing.is_some() {
                return Err(AppError::Conflict(format!(
                    "Email {} already registered",
                    email
                )));
            }
        }

        let customer = sqlx::query_as::<_, Customer>(
            r#"
            INSERT INTO customers (name, phone, email, address, total_spent, loyalty_points)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
            "#,
        )
        .bind(&request.name)
        .bind(&request.phone)
        .bind(&request.email)
        .bind(&request.address)
        .bind(Decimal::ZERO)
        .bind(0)
        .fetch_one(&self.db.pool)
        .await?;

        tracing::info!("Customer created: {} (ID: {})", customer.name, customer.id);
        Ok(customer)
    }

    /// Get customer by ID
    pub async fn get_customer(&self, id: i32) -> AppResult<Customer> {
        sqlx::query_as::<_, Customer>("SELECT * FROM customers WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.db.pool)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Customer with id {} not found", id)))
    }

    /// Get customer by phone number
    pub async fn get_customer_by_phone(&self, phone: &str) -> AppResult<Customer> {
        sqlx::query_as::<_, Customer>("SELECT * FROM customers WHERE phone = $1")
            .bind(phone)
            .fetch_optional(&self.db.pool)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Customer with phone {} not found", phone)))
    }

    /// Search customers by name or phone
    pub async fn search_customers(&self, query: &str) -> AppResult<Vec<Customer>> {
        let search_pattern = format!("%{}%", query);

        sqlx::query_as::<_, Customer>(
            r#"
            SELECT * FROM customers
            WHERE name ILIKE $1 OR phone ILIKE $1 OR email ILIKE $1
            ORDER BY name
            LIMIT 50
            "#,
        )
        .bind(&search_pattern)
        .fetch_all(&self.db.pool)
        .await
        .map_err(AppError::Database)
    }

    /// List all customers with pagination
    pub async fn list_customers(&self, limit: i64, offset: i64) -> AppResult<Vec<Customer>> {
        sqlx::query_as::<_, Customer>("SELECT * FROM customers ORDER BY name LIMIT $1 OFFSET $2")
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.db.pool)
            .await
            .map_err(AppError::Database)
    }

    /// Update customer
    pub async fn update_customer(
        &self,
        id: i32,
        request: UpdateCustomerRequest,
    ) -> AppResult<Customer> {
        // Check if customer exists
        let existing = self.get_customer(id).await?;

        // Validate phone uniqueness if changing
        if let Some(ref phone) = request.phone {
            if existing.phone.as_deref() != Some(phone) {
                let existing_phone = sqlx::query_scalar::<_, i32>(
                    "SELECT id FROM customers WHERE phone = $1 AND id != $2",
                )
                .bind(phone)
                .bind(id)
                .fetch_optional(&self.db.pool)
                .await?;

                if existing_phone.is_some() {
                    return Err(AppError::Conflict(format!(
                        "Phone number {} already registered",
                        phone
                    )));
                }
            }
        }

        // Validate email uniqueness if changing
        if let Some(ref email) = request.email {
            if existing.email.as_deref() != Some(email) {
                let existing_email = sqlx::query_scalar::<_, i32>(
                    "SELECT id FROM customers WHERE email = $1 AND id != $2",
                )
                .bind(email)
                .bind(id)
                .fetch_optional(&self.db.pool)
                .await?;

                if existing_email.is_some() {
                    return Err(AppError::Conflict(format!(
                        "Email {} already registered",
                        email
                    )));
                }
            }
        }

        let customer = sqlx::query_as::<_, Customer>(
            r#"
            UPDATE customers
            SET name = COALESCE($1, name),
                phone = COALESCE($2, phone),
                email = COALESCE($3, email),
                address = COALESCE($4, address),
                updated_at = CURRENT_TIMESTAMP
            WHERE id = $5
            RETURNING *
            "#,
        )
        .bind(&request.name)
        .bind(&request.phone)
        .bind(&request.email)
        .bind(&request.address)
        .bind(id)
        .fetch_one(&self.db.pool)
        .await?;

        tracing::info!("Customer updated: {} (ID: {})", customer.name, customer.id);
        Ok(customer)
    }

    /// Delete customer
    pub async fn delete_customer(&self, id: i32) -> AppResult<()> {
        // Check if customer has sales
        let has_sales = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM sales WHERE customer_id = $1)",
        )
        .bind(id)
        .fetch_one(&self.db.pool)
        .await?;

        if has_sales {
            return Err(AppError::Conflict(
                "Cannot delete customer with purchase history".to_string(),
            ));
        }

        let result = sqlx::query("DELETE FROM customers WHERE id = $1")
            .bind(id)
            .execute(&self.db.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!(
                "Customer with id {} not found",
                id
            )));
        }

        tracing::info!("Customer deleted: ID {}", id);
        Ok(())
    }

    /// Get customer purchase history
    pub async fn get_customer_purchase_history(
        &self,
        customer_id: i32,
        limit: i64,
    ) -> AppResult<Vec<crate::models::Sale>> {
        // Check if customer exists
        self.get_customer(customer_id).await?;

        sqlx::query_as::<_, crate::models::Sale>(
            "SELECT * FROM sales WHERE customer_id = $1 ORDER BY created_at DESC LIMIT $2",
        )
        .bind(customer_id)
        .bind(limit)
        .fetch_all(&self.db.pool)
        .await
        .map_err(AppError::Database)
    }

    /// Add loyalty points to customer
    pub async fn add_loyalty_points(&self, customer_id: i32, points: i32) -> AppResult<Customer> {
        let customer = sqlx::query_as::<_, Customer>(
            r#"
            UPDATE customers
            SET loyalty_points = loyalty_points + $1,
                updated_at = CURRENT_TIMESTAMP
            WHERE id = $2
            RETURNING *
            "#,
        )
        .bind(points)
        .bind(customer_id)
        .fetch_one(&self.db.pool)
        .await?;

        tracing::info!(
            "Added {} loyalty points to customer {} (ID: {})",
            points,
            customer.name,
            customer_id
        );
        Ok(customer)
    }

    /// Redeem loyalty points from customer
    pub async fn redeem_loyalty_points(
        &self,
        customer_id: i32,
        points: i32,
    ) -> AppResult<Customer> {
        let customer = self.get_customer(customer_id).await?;

        if customer.loyalty_points < points {
            return Err(AppError::Validation(format!(
                "Insufficient loyalty points. Available: {}, Required: {}",
                customer.loyalty_points, points
            )));
        }

        let updated_customer = sqlx::query_as::<_, Customer>(
            r#"
            UPDATE customers
            SET loyalty_points = loyalty_points - $1,
                updated_at = CURRENT_TIMESTAMP
            WHERE id = $2
            RETURNING *
            "#,
        )
        .bind(points)
        .bind(customer_id)
        .fetch_one(&self.db.pool)
        .await?;

        tracing::info!(
            "Redeemed {} loyalty points from customer {} (ID: {})",
            points,
            updated_customer.name,
            customer_id
        );
        Ok(updated_customer)
    }

    /// Get top customers by total spent
    pub async fn get_top_customers(&self, limit: i64) -> AppResult<Vec<Customer>> {
        sqlx::query_as::<_, Customer>("SELECT * FROM customers ORDER BY total_spent DESC LIMIT $1")
            .bind(limit)
            .fetch_all(&self.db.pool)
            .await
            .map_err(AppError::Database)
    }

    /// Get customers with loyalty points above threshold
    pub async fn get_customers_by_loyalty_points(
        &self,
        min_points: i32,
    ) -> AppResult<Vec<Customer>> {
        sqlx::query_as::<_, Customer>(
            "SELECT * FROM customers WHERE loyalty_points >= $1 ORDER BY loyalty_points DESC",
        )
        .bind(min_points)
        .fetch_all(&self.db.pool)
        .await
        .map_err(AppError::Database)
    }
}
