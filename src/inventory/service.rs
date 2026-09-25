use crate::database::Database;
use crate::error::{AppError, AppResult};
use crate::models::{
    AdjustmentType, Category, CreateCategoryRequest, CreateProductRequest,
    CreateProductVariantRequest, InventoryLog, Product, ProductVariant, StockAdjustmentRequest,
    UpdateCategoryRequest, UpdateProductRequest, UpdateProductVariantRequest,
};
use crate::utils::BranchContext;

pub struct InventoryService {
    db: Database,
}

impl InventoryService {
    pub fn new(db: Database) -> Self {
        InventoryService { db }
    }

    // ==================== Product Management ====================

    /// Create a new product
    pub async fn create_product(
        &self,
        request: CreateProductRequest,
        user_id: i32,
        branch_context: &BranchContext,
    ) -> AppResult<Product> {
        // Validate SKU uniqueness
        let existing = sqlx::query_scalar::<_, i32>("SELECT id FROM products WHERE sku = $1")
            .bind(&request.sku)
            .fetch_optional(&self.db.pool)
            .await?;

        if existing.is_some() {
            return Err(AppError::Conflict(format!(
                "SKU {} already exists",
                request.sku
            )));
        }

        // Validate barcode uniqueness if provided
        if let Some(ref barcode) = request.barcode {
            let existing =
                sqlx::query_scalar::<_, i32>("SELECT id FROM products WHERE barcode = $1")
                    .bind(barcode)
                    .fetch_optional(&self.db.pool)
                    .await?;

            if existing.is_some() {
                return Err(AppError::Conflict(format!(
                    "Barcode {} already exists",
                    barcode
                )));
            }
        }

        // Validate category exists if provided
        if let Some(category_id) = request.category_id {
            let exists = sqlx::query_scalar::<_, bool>(
                "SELECT EXISTS(SELECT 1 FROM categories WHERE id = $1)",
            )
            .bind(category_id)
            .fetch_one(&self.db.pool)
            .await?;

            if !exists {
                return Err(AppError::NotFound(format!(
                    "Category with id {} not found",
                    category_id
                )));
            }
        }

        let product = sqlx::query_as::<_, Product>(
            r#"
            INSERT INTO products (sku, barcode, name, description, category_id, cost_price, 
                                  selling_price, stock_quantity, low_stock_threshold, tax_rate, branch_id)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            RETURNING *
            "#,
        )
        .bind(&request.sku)
        .bind(&request.barcode)
        .bind(&request.name)
        .bind(&request.description)
        .bind(request.category_id)
        .bind(request.cost_price)
        .bind(request.selling_price)
        .bind(request.stock_quantity)
        .bind(request.low_stock_threshold)
        .bind(request.tax_rate)
        .bind(branch_context.get_filter_branch_id())
        .fetch_one(&self.db.pool)
        .await?;

        // Log initial stock
        if request.stock_quantity > 0 {
            self.log_stock_adjustment(
                product.id,
                user_id,
                request.stock_quantity,
                AdjustmentType::Restock,
                "Initial stock".to_string(),
            )
            .await?;
        }

        tracing::info!("Product created: {} (SKU: {})", product.name, product.sku);
        Ok(product)
    }

    /// Get product by ID
    pub async fn get_product(&self, id: i32) -> AppResult<Product> {
        sqlx::query_as::<_, Product>("SELECT * FROM products WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.db.pool)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Product with id {} not found", id)))
    }

    /// Get product by SKU
    pub async fn get_product_by_sku(&self, sku: &str) -> AppResult<Product> {
        sqlx::query_as::<_, Product>("SELECT * FROM products WHERE sku = $1")
            .bind(sku)
            .fetch_optional(&self.db.pool)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Product with SKU {} not found", sku)))
    }

    /// Get product by barcode
    pub async fn get_product_by_barcode(&self, barcode: &str) -> AppResult<Product> {
        sqlx::query_as::<_, Product>("SELECT * FROM products WHERE barcode = $1")
            .bind(barcode)
            .fetch_optional(&self.db.pool)
            .await?
            .ok_or_else(|| {
                AppError::NotFound(format!("Product with barcode {} not found", barcode))
            })
    }

    /// List all products with optional filters
    pub async fn list_products(
        &self,
        category_id: Option<i32>,
        active_only: bool,
        low_stock_only: bool,
    ) -> AppResult<Vec<Product>> {
        let mut query = String::from("SELECT * FROM products WHERE 1=1");
        let mut count = 0;

        if let Some(_cat_id) = category_id {
            count += 1;
            query.push_str(&format!(" AND category_id = ${}", count));
        }

        if active_only {
            count += 1;
            query.push_str(&format!(" AND is_active = true"));
        }

        if low_stock_only {
            count += 1;
            query.push_str(&format!(" AND stock_quantity <= low_stock_threshold"));
        }

        query.push_str(" ORDER BY name");

        let mut query_builder = sqlx::query_as::<_, Product>(&query);

        if let Some(cat_id) = category_id {
            query_builder = query_builder.bind(cat_id);
        }

        let products = query_builder.fetch_all(&self.db.pool).await?;
        Ok(products)
    }

    /// Update product
    pub async fn update_product(
        &self,
        id: i32,
        request: UpdateProductRequest,
    ) -> AppResult<Product> {
        // Check if product exists
        let existing = self.get_product(id).await?;

        // Build dynamic update query
        let mut updates = Vec::new();
        let mut count = 0;

        if let Some(barcode) = &request.barcode {
            // Check barcode uniqueness if changing
            if existing.barcode.as_deref() != Some(barcode) {
                let existing_barcode = sqlx::query_scalar::<_, i32>(
                    "SELECT id FROM products WHERE barcode = $1 AND id != $2",
                )
                .bind(barcode)
                .bind(id)
                .fetch_optional(&self.db.pool)
                .await?;

                if existing_barcode.is_some() {
                    return Err(AppError::Conflict(format!(
                        "Barcode {} already exists",
                        barcode
                    )));
                }
            }
            count += 1;
            updates.push(format!("barcode = ${}", count));
        }

        if let Some(_name) = &request.name {
            count += 1;
            updates.push(format!("name = ${}", count));
        }

        if let Some(_description) = &request.description {
            count += 1;
            updates.push(format!("description = ${}", count));
        }

        if let Some(category_id) = request.category_id {
            // Validate category exists
            let exists = sqlx::query_scalar::<_, bool>(
                "SELECT EXISTS(SELECT 1 FROM categories WHERE id = $1)",
            )
            .bind(category_id)
            .fetch_one(&self.db.pool)
            .await?;

            if !exists {
                return Err(AppError::NotFound(format!(
                    "Category with id {} not found",
                    category_id
                )));
            }
            count += 1;
            updates.push(format!("category_id = ${}", count));
        }

        if let Some(_cost_price) = request.cost_price {
            count += 1;
            updates.push(format!("cost_price = ${}", count));
        }

        if let Some(_selling_price) = request.selling_price {
            count += 1;
            updates.push(format!("selling_price = ${}", count));
        }

        if let Some(_stock_quantity) = request.stock_quantity {
            count += 1;
            updates.push(format!("stock_quantity = ${}", count));
        }

        if let Some(_low_stock_threshold) = request.low_stock_threshold {
            count += 1;
            updates.push(format!("low_stock_threshold = ${}", count));
        }

        if let Some(_tax_rate) = request.tax_rate {
            count += 1;
            updates.push(format!("tax_rate = ${}", count));
        }

        if let Some(_is_active) = request.is_active {
            count += 1;
            updates.push(format!("is_active = ${}", count));
        }

        if updates.is_empty() {
            return Ok(existing);
        }

        updates.push("updated_at = CURRENT_TIMESTAMP".to_string());

        let query = format!(
            "UPDATE products SET {} WHERE id = ${} RETURNING *",
            updates.join(", "),
            count + 1
        );

        let mut query_builder = sqlx::query_as::<_, Product>(&query);

        if let Some(barcode) = &request.barcode {
            query_builder = query_builder.bind(barcode);
        }
        if let Some(name) = &request.name {
            query_builder = query_builder.bind(name);
        }
        if let Some(description) = &request.description {
            query_builder = query_builder.bind(description);
        }
        if let Some(category_id) = request.category_id {
            query_builder = query_builder.bind(category_id);
        }
        if let Some(cost_price) = request.cost_price {
            query_builder = query_builder.bind(cost_price);
        }
        if let Some(selling_price) = request.selling_price {
            query_builder = query_builder.bind(selling_price);
        }
        if let Some(stock_quantity) = request.stock_quantity {
            query_builder = query_builder.bind(stock_quantity);
        }
        if let Some(low_stock_threshold) = request.low_stock_threshold {
            query_builder = query_builder.bind(low_stock_threshold);
        }
        if let Some(tax_rate) = request.tax_rate {
            query_builder = query_builder.bind(tax_rate);
        }
        if let Some(is_active) = request.is_active {
            query_builder = query_builder.bind(is_active);
        }

        query_builder = query_builder.bind(id);

        let product = query_builder.fetch_one(&self.db.pool).await?;

        tracing::info!("Product updated: {} (ID: {})", product.name, product.id);
        Ok(product)
    }

    /// Delete product (soft delete by setting is_active to false)
    pub async fn delete_product(&self, id: i32) -> AppResult<()> {
        let result = sqlx::query("UPDATE products SET is_active = false WHERE id = $1")
            .bind(id)
            .execute(&self.db.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!(
                "Product with id {} not found",
                id
            )));
        }

        tracing::info!("Product soft deleted: ID {}", id);
        Ok(())
    }

    /// Adjust stock quantity
    pub async fn adjust_stock(
        &self,
        id: i32,
        request: StockAdjustmentRequest,
        user_id: i32,
    ) -> AppResult<Product> {
        let product = self.get_product(id).await?;

        let new_quantity = product.stock_quantity + request.change_amount;

        if new_quantity < 0 {
            return Err(AppError::InsufficientStock(
                "Stock cannot be negative".to_string(),
            ));
        }

        // Update stock
        let updated_product = sqlx::query_as::<_, Product>(
            "UPDATE products SET stock_quantity = $1, updated_at = CURRENT_TIMESTAMP WHERE id = $2 RETURNING *",
        )
        .bind(new_quantity)
        .bind(id)
        .fetch_one(&self.db.pool)
        .await?;

        // Determine adjustment type
        let adjustment_type = if request.change_amount > 0 {
            AdjustmentType::Restock
        } else {
            AdjustmentType::ManualCorrection
        };

        // Log the adjustment
        self.log_stock_adjustment(
            id,
            user_id,
            request.change_amount,
            adjustment_type,
            request.reason,
        )
        .await?;

        tracing::info!(
            "Stock adjusted for product {}: {} -> {} ({})",
            product.name,
            product.stock_quantity,
            new_quantity,
            request.change_amount
        );

        Ok(updated_product)
    }

    /// Get low stock products
    pub async fn get_low_stock_products(&self) -> AppResult<Vec<Product>> {
        self.list_products(None, true, true).await
    }

    // ==================== Category Management ====================

    /// Create a new category
    pub async fn create_category(&self, request: CreateCategoryRequest) -> AppResult<Category> {
        // Validate parent category exists if provided
        if let Some(parent_id) = request.parent_id {
            let exists = sqlx::query_scalar::<_, bool>(
                "SELECT EXISTS(SELECT 1 FROM categories WHERE id = $1)",
            )
            .bind(parent_id)
            .fetch_one(&self.db.pool)
            .await?;

            if !exists {
                return Err(AppError::NotFound(format!(
                    "Parent category with id {} not found",
                    parent_id
                )));
            }
        }

        let category = sqlx::query_as::<_, Category>(
            r#"
            INSERT INTO categories (name, description, parent_id)
            VALUES ($1, $2, $3)
            RETURNING *
            "#,
        )
        .bind(&request.name)
        .bind(&request.description)
        .bind(request.parent_id)
        .fetch_one(&self.db.pool)
        .await?;

        tracing::info!("Category created: {}", category.name);
        Ok(category)
    }

    /// Get category by ID
    pub async fn get_category(&self, id: i32) -> AppResult<Category> {
        sqlx::query_as::<_, Category>("SELECT * FROM categories WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.db.pool)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Category with id {} not found", id)))
    }

    /// List all categories
    pub async fn list_categories(&self) -> AppResult<Vec<Category>> {
        sqlx::query_as::<_, Category>("SELECT * FROM categories ORDER BY name")
            .fetch_all(&self.db.pool)
            .await
            .map_err(AppError::Database)
    }

    /// Update category
    pub async fn update_category(
        &self,
        id: i32,
        request: UpdateCategoryRequest,
    ) -> AppResult<Category> {
        // Check if category exists
        let _existing = self.get_category(id).await?;

        // Validate parent category exists if provided
        if let Some(parent_id) = request.parent_id {
            if parent_id == id {
                return Err(AppError::Validation(
                    "Category cannot be its own parent".to_string(),
                ));
            }

            let exists = sqlx::query_scalar::<_, bool>(
                "SELECT EXISTS(SELECT 1 FROM categories WHERE id = $1)",
            )
            .bind(parent_id)
            .fetch_one(&self.db.pool)
            .await?;

            if !exists {
                return Err(AppError::NotFound(format!(
                    "Parent category with id {} not found",
                    parent_id
                )));
            }
        }

        let category = sqlx::query_as::<_, Category>(
            r#"
            UPDATE categories 
            SET name = COALESCE($1, name),
                description = COALESCE($2, description),
                parent_id = $3
            WHERE id = $4
            RETURNING *
            "#,
        )
        .bind(&request.name)
        .bind(&request.description)
        .bind(request.parent_id)
        .bind(id)
        .fetch_one(&self.db.pool)
        .await?;

        tracing::info!("Category updated: {}", category.name);
        Ok(category)
    }

    /// Delete category
    pub async fn delete_category(&self, id: i32) -> AppResult<()> {
        // Check if category has products
        let has_products = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM products WHERE category_id = $1)",
        )
        .bind(id)
        .fetch_one(&self.db.pool)
        .await?;

        if has_products {
            return Err(AppError::Conflict(
                "Cannot delete category with associated products".to_string(),
            ));
        }

        // Check if category has subcategories
        let has_subcategories = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM categories WHERE parent_id = $1)",
        )
        .bind(id)
        .fetch_one(&self.db.pool)
        .await?;

        if has_subcategories {
            return Err(AppError::Conflict(
                "Cannot delete category with subcategories".to_string(),
            ));
        }

        let result = sqlx::query("DELETE FROM categories WHERE id = $1")
            .bind(id)
            .execute(&self.db.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!(
                "Category with id {} not found",
                id
            )));
        }

        tracing::info!("Category deleted: ID {}", id);
        Ok(())
    }

    // ==================== Inventory Logging ====================

    /// Log stock adjustment
    async fn log_stock_adjustment(
        &self,
        product_id: i32,
        user_id: i32,
        change_amount: i32,
        adjustment_type: AdjustmentType,
        reason: String,
    ) -> AppResult<InventoryLog> {
        sqlx::query_as::<_, InventoryLog>(
            r#"
            INSERT INTO inventory_logs (product_id, user_id, change_amount, type, reason)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING *
            "#,
        )
        .bind(product_id)
        .bind(user_id)
        .bind(change_amount)
        .bind(adjustment_type)
        .bind(reason)
        .fetch_one(&self.db.pool)
        .await
        .map_err(AppError::Database)
    }

    /// Get inventory logs for a product
    pub async fn get_inventory_logs(&self, product_id: i32) -> AppResult<Vec<InventoryLog>> {
        sqlx::query_as::<_, InventoryLog>(
            "SELECT * FROM inventory_logs WHERE product_id = $1 ORDER BY created_at DESC",
        )
        .bind(product_id)
        .fetch_all(&self.db.pool)
        .await
        .map_err(AppError::Database)
    }

    /// Get recent inventory logs
    pub async fn get_recent_inventory_logs(&self, limit: i64) -> AppResult<Vec<InventoryLog>> {
        sqlx::query_as::<_, InventoryLog>(
            "SELECT * FROM inventory_logs ORDER BY created_at DESC LIMIT $1",
        )
        .bind(limit)
        .fetch_all(&self.db.pool)
        .await
        .map_err(AppError::Database)
    }

    // ==================== Product Variants ====================

    /// Create a variant (size/color option) for an existing product.
    pub async fn create_variant(
        &self,
        product_id: i32,
        request: CreateProductVariantRequest,
    ) -> AppResult<ProductVariant> {
        // Parent product must exist.
        let product_exists =
            sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM products WHERE id = $1)")
                .bind(product_id)
                .fetch_one(&self.db.pool)
                .await?;

        if !product_exists {
            return Err(AppError::NotFound(format!(
                "Product with id {} not found",
                product_id
            )));
        }

        // SKU must be globally unique.
        let sku_taken = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM product_variants WHERE sku = $1)",
        )
        .bind(&request.sku)
        .fetch_one(&self.db.pool)
        .await?;

        if sku_taken {
            return Err(AppError::Conflict(format!(
                "Variant SKU {} already exists",
                request.sku
            )));
        }

        // Barcode must be unique when provided.
        if let Some(ref barcode) = request.barcode {
            let barcode_taken = sqlx::query_scalar::<_, bool>(
                "SELECT EXISTS(SELECT 1 FROM product_variants WHERE barcode = $1)",
            )
            .bind(barcode)
            .fetch_one(&self.db.pool)
            .await?;

            if barcode_taken {
                return Err(AppError::Conflict(format!(
                    "Barcode {} already exists",
                    barcode
                )));
            }
        }

        let size = request.size.unwrap_or_default();
        let color = request.color.unwrap_or_default();

        let variant = sqlx::query_as::<_, ProductVariant>(
            r#"
            INSERT INTO product_variants (product_id, sku, barcode, name, size, color,
                                          cost_price, selling_price, stock_quantity, low_stock_threshold)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING *
            "#,
        )
        .bind(product_id)
        .bind(&request.sku)
        .bind(&request.barcode)
        .bind(&request.name)
        .bind(&size)
        .bind(&color)
        .bind(request.cost_price)
        .bind(request.selling_price)
        .bind(request.stock_quantity)
        .bind(request.low_stock_threshold)
        .fetch_one(&self.db.pool)
        .await
        .map_err(|e| match e {
            // Unique violation on (product_id, size, color)
            sqlx::Error::Database(ref db) if db.code().as_deref() == Some("23505") => {
                AppError::Conflict(format!(
                    "A variant with size '{}' and color '{}' already exists for this product",
                    size, color
                ))
            }
            other => AppError::Database(other),
        })?;

        tracing::info!(
            "Variant created for product {}: {} (SKU: {})",
            product_id,
            variant.name,
            variant.sku
        );
        Ok(variant)
    }

    /// List all variants belonging to a product.
    pub async fn list_variants(&self, product_id: i32) -> AppResult<Vec<ProductVariant>> {
        sqlx::query_as::<_, ProductVariant>(
            "SELECT * FROM product_variants WHERE product_id = $1 ORDER BY size, color",
        )
        .bind(product_id)
        .fetch_all(&self.db.pool)
        .await
        .map_err(AppError::Database)
    }

    /// Get a single variant by id.
    pub async fn get_variant(&self, id: i32) -> AppResult<ProductVariant> {
        sqlx::query_as::<_, ProductVariant>("SELECT * FROM product_variants WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.db.pool)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Variant {} not found", id)))
    }

    /// Update a variant. Only the provided fields change (COALESCE keeps the rest).
    pub async fn update_variant(
        &self,
        id: i32,
        request: UpdateProductVariantRequest,
    ) -> AppResult<ProductVariant> {
        sqlx::query_as::<_, ProductVariant>(
            r#"
            UPDATE product_variants SET
                barcode = COALESCE($2, barcode),
                name = COALESCE($3, name),
                size = COALESCE($4, size),
                color = COALESCE($5, color),
                cost_price = COALESCE($6, cost_price),
                selling_price = COALESCE($7, selling_price),
                stock_quantity = COALESCE($8, stock_quantity),
                low_stock_threshold = COALESCE($9, low_stock_threshold),
                is_active = COALESCE($10, is_active)
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(&request.barcode)
        .bind(&request.name)
        .bind(&request.size)
        .bind(&request.color)
        .bind(request.cost_price)
        .bind(request.selling_price)
        .bind(request.stock_quantity)
        .bind(request.low_stock_threshold)
        .bind(request.is_active)
        .fetch_optional(&self.db.pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Variant {} not found", id)))
    }

    /// Delete a variant by id.
    pub async fn delete_variant(&self, id: i32) -> AppResult<()> {
        let result = sqlx::query("DELETE FROM product_variants WHERE id = $1")
            .bind(id)
            .execute(&self.db.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("Variant {} not found", id)));
        }

        tracing::info!("Variant {} deleted", id);
        Ok(())
    }
}

/// Integration tests for the product-variant service methods.
///
/// These hit a real PostgreSQL database, so they are `#[ignore]`d to keep the
/// default `cargo test` run (and CI, which has no database) green. No Docker is
/// used — point `TEST_DATABASE_URL` (or `DATABASE_URL`) at a disposable local
/// Postgres and run:  cargo test -- --ignored
#[cfg(test)]
mod variant_integration_tests {
    use super::*;
    use rust_decimal_macros::dec;
    use sqlx::postgres::PgPoolOptions;
    use std::time::{SystemTime, UNIX_EPOCH};

    async fn setup() -> Database {
        let url = std::env::var("TEST_DATABASE_URL")
            .or_else(|_| std::env::var("DATABASE_URL"))
            .expect("set TEST_DATABASE_URL or DATABASE_URL to run variant integration tests");
        let pool = PgPoolOptions::new()
            .max_connections(1)
            .connect(&url)
            .await
            .expect("connect to test database");
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .expect("run migrations");
        Database { pool }
    }

    fn unique(prefix: &str) -> String {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        format!("{}-{}", prefix, nanos)
    }

    async fn create_parent_product(db: &Database) -> i32 {
        let sku = unique("P");
        sqlx::query_scalar::<_, i32>(
            r#"INSERT INTO products
                   (sku, name, cost_price, selling_price, stock_quantity, low_stock_threshold, tax_rate)
               VALUES ($1, $2, 0, 0, 0, 0, 0)
               RETURNING id"#,
        )
        .bind(&sku)
        .bind(format!("Parent {}", sku))
        .fetch_one(&db.pool)
        .await
        .expect("insert parent product")
    }

    fn create_request(sku: &str, size: &str, color: &str) -> CreateProductVariantRequest {
        CreateProductVariantRequest {
            sku: sku.into(),
            barcode: None,
            name: format!("Variant {}", sku),
            size: Some(size.into()),
            color: Some(color.into()),
            cost_price: dec!(5.00),
            selling_price: dec!(10.00),
            stock_quantity: 20,
            low_stock_threshold: 5,
        }
    }

    fn empty_update() -> UpdateProductVariantRequest {
        UpdateProductVariantRequest {
            barcode: None,
            name: None,
            size: None,
            color: None,
            cost_price: None,
            selling_price: None,
            stock_quantity: None,
            low_stock_threshold: None,
            is_active: None,
        }
    }

    #[tokio::test]
    #[ignore = "requires a real Postgres (TEST_DATABASE_URL/DATABASE_URL); run with `cargo test -- --ignored`"]
    async fn create_variant_persists_row() {
        let db = setup().await;
        let parent = create_parent_product(&db).await;
        let svc = InventoryService::new(db);

        let sku = unique("V");
        let variant = svc
            .create_variant(parent, create_request(&sku, "L", "Red"))
            .await
            .expect("create variant");

        assert_eq!(variant.product_id, parent);
        assert_eq!(variant.sku, sku);
        assert_eq!(variant.size, "L");
        assert_eq!(variant.color, "Red");
        assert!(variant.is_active);
    }

    #[tokio::test]
    #[ignore = "requires a real Postgres (TEST_DATABASE_URL/DATABASE_URL); run with `cargo test -- --ignored`"]
    async fn create_variant_defaults_empty_size_and_color() {
        let db = setup().await;
        let parent = create_parent_product(&db).await;
        let svc = InventoryService::new(db);

        let mut req = create_request(&unique("V"), "", "");
        req.size = None;
        req.color = None;
        let variant = svc.create_variant(parent, req).await.expect("create");

        assert_eq!(variant.size, "");
        assert_eq!(variant.color, "");
    }

    #[tokio::test]
    #[ignore = "requires a real Postgres (TEST_DATABASE_URL/DATABASE_URL); run with `cargo test -- --ignored`"]
    async fn create_variant_missing_product_is_not_found() {
        let db = setup().await;
        let svc = InventoryService::new(db);

        let err = svc
            .create_variant(999_999_999, create_request(&unique("V"), "M", "Blue"))
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::NotFound(_)));
    }

    #[tokio::test]
    #[ignore = "requires a real Postgres (TEST_DATABASE_URL/DATABASE_URL); run with `cargo test -- --ignored`"]
    async fn create_variant_duplicate_sku_is_conflict() {
        let db = setup().await;
        let parent = create_parent_product(&db).await;
        let svc = InventoryService::new(db);

        let sku = unique("V");
        svc.create_variant(parent, create_request(&sku, "S", "Green"))
            .await
            .expect("first create");
        let err = svc
            .create_variant(parent, create_request(&sku, "M", "Green"))
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::Conflict(_)));
    }

    #[tokio::test]
    #[ignore = "requires a real Postgres (TEST_DATABASE_URL/DATABASE_URL); run with `cargo test -- --ignored`"]
    async fn create_variant_duplicate_option_combination_is_conflict() {
        let db = setup().await;
        let parent = create_parent_product(&db).await;
        let svc = InventoryService::new(db);

        // Same (product_id, size, color) but distinct SKUs -> unique constraint -> Conflict.
        svc.create_variant(parent, create_request(&unique("V"), "XL", "Black"))
            .await
            .expect("first create");
        let err = svc
            .create_variant(parent, create_request(&unique("V"), "XL", "Black"))
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::Conflict(_)));
    }

    #[tokio::test]
    #[ignore = "requires a real Postgres (TEST_DATABASE_URL/DATABASE_URL); run with `cargo test -- --ignored`"]
    async fn list_and_get_variant_roundtrip() {
        let db = setup().await;
        let parent = create_parent_product(&db).await;
        let svc = InventoryService::new(db);

        let sku = unique("V");
        let created = svc
            .create_variant(parent, create_request(&sku, "M", "Navy"))
            .await
            .expect("create");

        let listed = svc.list_variants(parent).await.expect("list");
        assert!(listed.iter().any(|v| v.id == created.id));

        let fetched = svc.get_variant(created.id).await.expect("get");
        assert_eq!(fetched.sku, sku);
    }

    #[tokio::test]
    #[ignore = "requires a real Postgres (TEST_DATABASE_URL/DATABASE_URL); run with `cargo test -- --ignored`"]
    async fn get_missing_variant_is_not_found() {
        let db = setup().await;
        let svc = InventoryService::new(db);
        let err = svc.get_variant(999_999_999).await.unwrap_err();
        assert!(matches!(err, AppError::NotFound(_)));
    }

    #[tokio::test]
    #[ignore = "requires a real Postgres (TEST_DATABASE_URL/DATABASE_URL); run with `cargo test -- --ignored`"]
    async fn update_variant_applies_only_provided_fields() {
        let db = setup().await;
        let parent = create_parent_product(&db).await;
        let svc = InventoryService::new(db);

        let created = svc
            .create_variant(parent, create_request(&unique("V"), "S", "White"))
            .await
            .expect("create");

        let mut req = empty_update();
        req.name = Some("Renamed".into());
        req.selling_price = Some(dec!(15.00));
        req.stock_quantity = Some(3);
        req.is_active = Some(false);

        let updated = svc.update_variant(created.id, req).await.expect("update");
        assert_eq!(updated.name, "Renamed");
        assert_eq!(updated.selling_price, dec!(15.00));
        assert_eq!(updated.stock_quantity, 3);
        assert!(!updated.is_active);
        // Untouched fields are preserved by COALESCE.
        assert_eq!(updated.size, "S");
        assert_eq!(updated.color, "White");
        assert_eq!(updated.cost_price, dec!(5.00));
    }

    #[tokio::test]
    #[ignore = "requires a real Postgres (TEST_DATABASE_URL/DATABASE_URL); run with `cargo test -- --ignored`"]
    async fn update_missing_variant_is_not_found() {
        let db = setup().await;
        let svc = InventoryService::new(db);

        let mut req = empty_update();
        req.name = Some("x".into());
        let err = svc.update_variant(999_999_999, req).await.unwrap_err();
        assert!(matches!(err, AppError::NotFound(_)));
    }

    #[tokio::test]
    #[ignore = "requires a real Postgres (TEST_DATABASE_URL/DATABASE_URL); run with `cargo test -- --ignored`"]
    async fn delete_variant_removes_row() {
        let db = setup().await;
        let parent = create_parent_product(&db).await;
        let svc = InventoryService::new(db);

        let created = svc
            .create_variant(parent, create_request(&unique("V"), "XS", "Teal"))
            .await
            .expect("create");

        svc.delete_variant(created.id).await.expect("delete");

        let err = svc.get_variant(created.id).await.unwrap_err();
        assert!(matches!(err, AppError::NotFound(_)));

        // Deleting again reports NotFound (rows_affected == 0).
        let err2 = svc.delete_variant(created.id).await.unwrap_err();
        assert!(matches!(err2, AppError::NotFound(_)));
    }

    #[tokio::test]
    #[ignore = "requires a real Postgres (TEST_DATABASE_URL/DATABASE_URL); run with `cargo test -- --ignored`"]
    async fn delete_missing_variant_is_not_found() {
        let db = setup().await;
        let svc = InventoryService::new(db);
        let err = svc.delete_variant(999_999_999).await.unwrap_err();
        assert!(matches!(err, AppError::NotFound(_)));
    }
}
