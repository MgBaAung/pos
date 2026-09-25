use crate::database::Database;
use crate::error::{AppError, AppResult};
use crate::models::{
    AdjustmentType, CashDrawerSession, CloseCashDrawerSessionRequest,
    CreateCashDrawerSessionRequest, CreateSaleRequest, PaymentMethod, Product, ReceiptData,
    ReceiptItem, Sale, SaleItem, SaleStatus, User,
};
use crate::restaurant::domain::kitchen_orders::{KitchenOrderEvent, KotStatus, OrderType};
use crate::restaurant::realtime::global_kitchen_sender;
use chrono::Utc;
use rust_decimal::prelude::*;
use rust_decimal::Decimal;
use serde_json::{json, Value};
use sqlx::Row;
use uuid::Uuid;

/// Each loyalty point is worth 0.01 currency when redeemed at checkout.
const LOYALTY_POINT_VALUE: Decimal = Decimal::from_parts(1, 0, 0, false, 2);

pub struct SalesService {
    db: Database,
}

/// A sale line after product lookup and modifier pricing, ready to persist.
struct PreparedItem {
    product_id: i32,
    product_name: String,
    quantity: i32,
    unit_price: Decimal,
    cost_price: Decimal,
    discount: Decimal,
    subtotal: Decimal,
    selected_modifiers: Value,
}

impl SalesService {
    pub fn new(db: Database) -> Self {
        SalesService { db }
    }

    // ==================== Sales Processing ====================

    /// Create a new sale with transaction safety
    pub async fn create_sale(
        &self,
        request: CreateSaleRequest,
        cashier_id: i32,
    ) -> AppResult<Sale> {
        let mut transaction = self.db.begin().await?;

        // Validate payment amount
        if request.amount_paid < Decimal::ZERO {
            return Err(AppError::Validation(
                "Amount paid must be positive".to_string(),
            ));
        }

        // Validate customer exists (if provided) and capture current loyalty points
        let mut customer_points = 0;
        if let Some(customer_id) = request.customer_id {
            customer_points =
                sqlx::query_scalar::<_, i32>("SELECT loyalty_points FROM customers WHERE id = $1")
                    .bind(customer_id)
                    .fetch_optional(&mut *transaction)
                    .await?
                    .ok_or_else(|| {
                        AppError::NotFound(format!("Customer with id {} not found", customer_id))
                    })?;
        }

        // Validate loyalty redemption request
        let points_redeemed = request.redeem_points.unwrap_or(0);
        if points_redeemed > 0 {
            if request.customer_id.is_none() {
                return Err(AppError::Validation(
                    "A customer is required to redeem loyalty points".to_string(),
                ));
            }
            if points_redeemed > customer_points {
                return Err(AppError::Validation(format!(
                    "Insufficient loyalty points. Available: {}, Requested: {}",
                    customer_points, points_redeemed
                )));
            }
        }
        let loyalty_discount = Decimal::from(points_redeemed) * LOYALTY_POINT_VALUE;

        // Validate table exists if this is a dine-in/takeaway order linked to a table
        if let Some(table_id) = request.table_id {
            let table_exists = sqlx::query_scalar::<_, bool>(
                "SELECT EXISTS(SELECT 1 FROM restaurant_tables WHERE id = $1)",
            )
            .bind(table_id)
            .fetch_one(&mut *transaction)
            .await?;

            if !table_exists {
                return Err(AppError::NotFound(format!(
                    "Restaurant table with id {} not found",
                    table_id
                )));
            }
        }

        // Infer order type when the client didn't state one: a sale attached to a
        // table is dine-in, otherwise it's a plain retail POS sale. Keeping retail
        // distinct from dine-in stops POS-only sales from being mislabeled in reports.
        let order_type = request.order_type.unwrap_or(if request.table_id.is_some() {
            OrderType::DineIn
        } else {
            OrderType::Retail
        });
        let guest_count = request.guest_count.unwrap_or(1);

        // Generate unique invoice number
        let invoice_no = format!(
            "INV-{}",
            Uuid::new_v4().to_string().replace("-", "")[..12].to_uppercase()
        );

        // Process sale items and calculate totals
        let mut subtotal = Decimal::ZERO;
        let mut tax_total = Decimal::ZERO;
        let discount_total = request.discount_total.unwrap_or(Decimal::ZERO);
        let mut prepared_items: Vec<PreparedItem> = Vec::new();

        for item_request in &request.items {
            // Get product with row lock to prevent race conditions
            let product =
                sqlx::query_as::<_, Product>("SELECT * FROM products WHERE id = $1 FOR UPDATE")
                    .bind(item_request.product_id)
                    .fetch_optional(&mut *transaction)
                    .await?
                    .ok_or_else(|| {
                        AppError::NotFound(format!(
                            "Product with id {} not found",
                            item_request.product_id
                        ))
                    })?;

            // Check stock availability
            if product.stock_quantity < item_request.quantity {
                return Err(AppError::InsufficientStock(format!(
                    "Insufficient stock for product {}. Available: {}, Required: {}",
                    product.name, product.stock_quantity, item_request.quantity
                )));
            }

            // Resolve selected modifiers: they must exist, be available, and be
            // linked to this product. Their price adjustments fold into unit_price.
            let mut unit_price = product.selling_price;
            let mut selected_modifiers = json!([]);
            if let Some(modifier_ids) = &item_request.selected_modifiers {
                if !modifier_ids.is_empty() {
                    let modifier_rows = sqlx::query(
                        r#"
                        SELECT m.id, m.name, m.price_adjustment
                        FROM modifiers m
                        JOIN product_modifier_groups pmg ON pmg.modifier_group_id = m.group_id
                        WHERE m.id = ANY($1) AND m.is_available = TRUE AND pmg.product_id = $2
                        "#,
                    )
                    .bind(modifier_ids.as_slice())
                    .bind(product.id)
                    .fetch_all(&mut *transaction)
                    .await?;

                    if modifier_rows.len() != modifier_ids.len() {
                        return Err(AppError::Validation(
                            "One or more selected modifiers are invalid or unavailable for this product"
                                .to_string(),
                        ));
                    }

                    let mut modifier_list = Vec::new();
                    let mut adjustment_total = Decimal::ZERO;
                    for row in &modifier_rows {
                        let mid: i32 = row.get("id");
                        let mname: String = row.get("name");
                        let madj: Decimal = row.get("price_adjustment");
                        adjustment_total += madj;
                        modifier_list.push(json!({
                            "id": mid,
                            "name": mname,
                            "price_adjustment": madj,
                        }));
                    }
                    unit_price += adjustment_total;
                    selected_modifiers = Value::Array(modifier_list);
                }
            }

            // Calculate item price (modifier adjustments are already in unit_price)
            let item_discount = item_request.discount.unwrap_or(Decimal::ZERO);
            let discounted_price = unit_price * (Decimal::ONE - item_discount / Decimal::from(100));
            let item_subtotal = discounted_price * Decimal::from(item_request.quantity);
            let item_tax = item_subtotal * (product.tax_rate / Decimal::from(100));

            subtotal += item_subtotal;
            tax_total += item_tax;

            // Update product stock
            let new_stock = product.stock_quantity - item_request.quantity;
            sqlx::query("UPDATE products SET stock_quantity = $1, updated_at = CURRENT_TIMESTAMP WHERE id = $2")
                .bind(new_stock)
                .bind(product.id)
                .execute(&mut *transaction)
                .await?;

            // Log stock adjustment
            sqlx::query(
                r#"
                INSERT INTO inventory_logs (product_id, user_id, change_amount, type, reason)
                VALUES ($1, $2, $3, $4, $5)
                "#,
            )
            .bind(product.id)
            .bind(cashier_id)
            .bind(-item_request.quantity)
            .bind(AdjustmentType::Sale)
            .bind(format!("Sale {}", invoice_no))
            .execute(&mut *transaction)
            .await?;

            prepared_items.push(PreparedItem {
                product_id: product.id,
                product_name: product.name.clone(),
                quantity: item_request.quantity,
                unit_price,
                cost_price: product.cost_price,
                discount: item_discount,
                subtotal: item_subtotal,
                selected_modifiers,
            });
        }

        let grand_total = subtotal + tax_total - discount_total - loyalty_discount;

        // Validate grand total is positive
        if grand_total <= Decimal::ZERO {
            return Err(AppError::Validation(
                "Grand total must be positive".to_string(),
            ));
        }

        // Validate payment amount covers grand total
        if request.amount_paid < grand_total {
            return Err(AppError::Payment(format!(
                "Insufficient payment. Required: {}, Paid: {}",
                grand_total, request.amount_paid
            )));
        }

        let change_given = request.amount_paid - grand_total;

        // Create sale record
        let sale = sqlx::query_as::<_, Sale>(
            r#"
            INSERT INTO sales (invoice_no, cashier_id, customer_id, subtotal, tax_total, 
                              discount_total, grand_total, amount_paid, change_given, 
                              payment_method, status, table_id, order_type, guest_count,
                              special_requests, loyalty_discount, points_redeemed)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17)
            RETURNING *
            "#,
        )
        .bind(&invoice_no)
        .bind(cashier_id)
        .bind(request.customer_id)
        .bind(subtotal)
        .bind(tax_total)
        .bind(discount_total)
        .bind(grand_total)
        .bind(request.amount_paid)
        .bind(change_given)
        .bind(request.payment_method)
        .bind(SaleStatus::Completed)
        .bind(request.table_id)
        .bind(order_type)
        .bind(guest_count)
        .bind(&request.special_requests)
        .bind(loyalty_discount)
        .bind(points_redeemed)
        .fetch_one(&mut *transaction)
        .await?;

        // Create sale items
        for item in &prepared_items {
            sqlx::query_as::<_, SaleItem>(
                r#"
                INSERT INTO sale_items (sale_id, product_id, quantity, unit_price, cost_price,
                                        discount, subtotal, selected_modifiers)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                RETURNING *
                "#,
            )
            .bind(sale.id)
            .bind(item.product_id)
            .bind(item.quantity)
            .bind(item.unit_price)
            .bind(item.cost_price)
            .bind(item.discount)
            .bind(item.subtotal)
            .bind(&item.selected_modifiers)
            .fetch_one(&mut *transaction)
            .await?;
        }

        // Restaurant flow: a sale that carries a table or an explicit order type is
        // treated as a kitchen order. Mark the table occupied and raise a KOT so it
        // shows up on the kitchen display in real time.
        let mut pending_kitchen_event: Option<KitchenOrderEvent> = None;
        let wants_kitchen = request.table_id.is_some() || request.order_type.is_some();
        if wants_kitchen {
            if request.table_id.is_some() && matches!(order_type, OrderType::DineIn) {
                sqlx::query(
                    "UPDATE restaurant_tables SET status = 'occupied', updated_at = CURRENT_TIMESTAMP WHERE id = $1",
                )
                .bind(request.table_id)
                .execute(&mut *transaction)
                .await?;
            }

            let table_number: Option<String> = match request.table_id {
                Some(table_id) => {
                    sqlx::query_scalar::<_, String>(
                        "SELECT table_number FROM restaurant_tables WHERE id = $1",
                    )
                    .bind(table_id)
                    .fetch_optional(&mut *transaction)
                    .await?
                }
                None => None,
            };

            let kitchen_order_id: i32 = sqlx::query_scalar(
                r#"
                INSERT INTO kitchen_orders (sale_id, table_id, order_type, status, priority, notes)
                VALUES ($1, $2, $3, 'pending', 5, $4)
                RETURNING id
                "#,
            )
            .bind(sale.id)
            .bind(request.table_id)
            .bind(order_type)
            .bind(&request.special_requests)
            .fetch_one(&mut *transaction)
            .await?;

            for (index, item) in prepared_items.iter().enumerate() {
                sqlx::query(
                    r#"
                    INSERT INTO kitchen_order_items
                        (kitchen_order_id, product_id, product_name, quantity, unit_price,
                         selected_modifiers, sequence_order)
                    VALUES ($1, $2, $3, $4, $5, $6, $7)
                    "#,
                )
                .bind(kitchen_order_id)
                .bind(item.product_id)
                .bind(&item.product_name)
                .bind(item.quantity)
                .bind(item.unit_price)
                .bind(&item.selected_modifiers)
                .bind(index as i32)
                .execute(&mut *transaction)
                .await?;
            }

            pending_kitchen_event = Some(KitchenOrderEvent {
                event_type: "kitchen_order_created".to_string(),
                kitchen_order_id,
                status: KotStatus::Pending,
                table_id: request.table_id,
                table_number,
                timestamp: Utc::now(),
            });
        }

        // Update customer total spent and loyalty points if customer provided
        if let Some(customer_id) = request.customer_id {
            let loyalty_points_earned = (grand_total / Decimal::from(10))
                .floor()
                .to_i32()
                .unwrap_or(0); // 1 point per $10
            let net_points = loyalty_points_earned - points_redeemed;

            sqlx::query(
                r#"
                UPDATE customers 
                SET total_spent = total_spent + $1,
                    loyalty_points = loyalty_points + $2,
                    updated_at = CURRENT_TIMESTAMP
                WHERE id = $3
                "#,
            )
            .bind(grand_total)
            .bind(net_points)
            .bind(customer_id)
            .execute(&mut *transaction)
            .await?;
        }

        // Commit transaction
        transaction.commit().await?;

        // Notify kitchen displays after the order is durably committed.
        if let Some(event) = pending_kitchen_event {
            if let Some(sender) = global_kitchen_sender() {
                let _ = sender.send(event);
            }
        }

        tracing::info!("Sale created: {} (Invoice: {})", sale.id, sale.invoice_no);
        Ok(sale)
    }

    /// Get sale by ID
    pub async fn get_sale(&self, id: i32) -> AppResult<Sale> {
        sqlx::query_as::<_, Sale>("SELECT * FROM sales WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.db.pool)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Sale with id {} not found", id)))
    }

    /// Get sale by invoice number
    pub async fn get_sale_by_invoice(&self, invoice_no: &str) -> AppResult<Sale> {
        sqlx::query_as::<_, Sale>("SELECT * FROM sales WHERE invoice_no = $1")
            .bind(invoice_no)
            .fetch_optional(&self.db.pool)
            .await?
            .ok_or_else(|| {
                AppError::NotFound(format!("Sale with invoice {} not found", invoice_no))
            })
    }

    /// List sales with filters
    pub async fn list_sales(
        &self,
        cashier_id: Option<i32>,
        customer_id: Option<i32>,
        status: Option<SaleStatus>,
        payment_method: Option<PaymentMethod>,
        limit: i64,
        offset: i64,
    ) -> AppResult<Vec<Sale>> {
        let mut query = String::from("SELECT * FROM sales WHERE 1=1");
        let mut count = 0;

        if let Some(_cashier) = cashier_id {
            count += 1;
            query.push_str(&format!(" AND cashier_id = ${}", count));
        }

        if let Some(_customer) = customer_id {
            count += 1;
            query.push_str(&format!(" AND customer_id = ${}", count));
        }

        if let Some(_s) = status {
            count += 1;
            query.push_str(&format!(" AND status = ${}", count));
        }

        if let Some(_pm) = payment_method {
            count += 1;
            query.push_str(&format!(" AND payment_method = ${}", count));
        }

        query.push_str(" ORDER BY created_at DESC");
        query.push_str(&format!(" LIMIT ${} OFFSET ${}", count + 1, count + 2));

        let mut query_builder = sqlx::query_as::<_, Sale>(&query);

        if let Some(cashier) = cashier_id {
            query_builder = query_builder.bind(cashier);
        }
        if let Some(customer) = customer_id {
            query_builder = query_builder.bind(customer);
        }
        if let Some(s) = status {
            query_builder = query_builder.bind(s);
        }
        if let Some(pm) = payment_method {
            query_builder = query_builder.bind(pm);
        }

        query_builder = query_builder.bind(limit).bind(offset);

        query_builder
            .fetch_all(&self.db.pool)
            .await
            .map_err(AppError::Database)
    }

    /// Get sale items for a sale
    pub async fn get_sale_items(&self, sale_id: i32) -> AppResult<Vec<SaleItem>> {
        sqlx::query_as::<_, SaleItem>("SELECT * FROM sale_items WHERE sale_id = $1 ORDER BY id")
            .bind(sale_id)
            .fetch_all(&self.db.pool)
            .await
            .map_err(AppError::Database)
    }

    /// Refund a sale (creates negative sale and restores stock)
    pub async fn refund_sale(&self, sale_id: i32, user_id: i32) -> AppResult<Sale> {
        let mut transaction = self.db.begin().await?;

        // Get original sale
        let original_sale =
            sqlx::query_as::<_, Sale>("SELECT * FROM sales WHERE id = $1 FOR UPDATE")
                .bind(sale_id)
                .fetch_optional(&mut *transaction)
                .await?
                .ok_or_else(|| AppError::NotFound(format!("Sale with id {} not found", sale_id)))?;

        // Check if already refunded
        if original_sale.status == SaleStatus::Refunded {
            return Err(AppError::Conflict("Sale already refunded".to_string()));
        }

        // Get sale items
        let sale_items =
            sqlx::query_as::<_, SaleItem>("SELECT * FROM sale_items WHERE sale_id = $1")
                .bind(sale_id)
                .fetch_all(&mut *transaction)
                .await?;

        // Restore stock for each item
        for item in sale_items {
            if let Some(product_id) = item.product_id {
                // Get product with row lock
                let product =
                    sqlx::query_as::<_, Product>("SELECT * FROM products WHERE id = $1 FOR UPDATE")
                        .bind(product_id)
                        .fetch_one(&mut *transaction)
                        .await?;

                // Restore stock
                let new_stock = product.stock_quantity + item.quantity;
                sqlx::query("UPDATE products SET stock_quantity = $1, updated_at = CURRENT_TIMESTAMP WHERE id = $2")
                    .bind(new_stock)
                    .bind(product_id)
                    .execute(&mut *transaction)
                    .await?;

                // Log stock adjustment
                sqlx::query(
                    r#"
                    INSERT INTO inventory_logs (product_id, user_id, change_amount, type, reason)
                    VALUES ($1, $2, $3, $4, $5)
                    "#,
                )
                .bind(product_id)
                .bind(user_id)
                .bind(item.quantity)
                .bind(AdjustmentType::ManualCorrection)
                .bind(format!("Refund for sale {}", original_sale.invoice_no))
                .execute(&mut *transaction)
                .await?;
            }
        }

        // Update sale status to refunded
        let refunded_sale =
            sqlx::query_as::<_, Sale>("UPDATE sales SET status = $1 WHERE id = $2 RETURNING *")
                .bind(SaleStatus::Refunded)
                .bind(sale_id)
                .fetch_one(&mut *transaction)
                .await?;

        // Commit transaction
        transaction.commit().await?;

        tracing::info!(
            "Sale refunded: {} (Invoice: {})",
            sale_id,
            original_sale.invoice_no
        );
        Ok(refunded_sale)
    }

    /// Generate receipt data for a sale
    pub async fn generate_receipt(&self, sale_id: i32) -> AppResult<ReceiptData> {
        let sale = self.get_sale(sale_id).await?;
        let sale_items = self.get_sale_items(sale_id).await?;

        // Get cashier info
        let cashier = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
            .bind(sale.cashier_id)
            .fetch_optional(&self.db.pool)
            .await?;

        let cashier_name = cashier
            .map(|u| u.full_name)
            .unwrap_or_else(|| "Unknown".to_string());

        // Get customer info if available
        let customer_name = if let Some(customer_id) = sale.customer_id {
            let customer = sqlx::query("SELECT name FROM customers WHERE id = $1")
                .bind(customer_id)
                .fetch_optional(&self.db.pool)
                .await
                .map_err(AppError::Database)?;
            customer.and_then(|row| row.try_get("name").ok())
        } else {
            None
        };

        // Build receipt items
        let mut receipt_items = Vec::new();
        for item in sale_items {
            if let Some(product_id) = item.product_id {
                let product = sqlx::query("SELECT name FROM products WHERE id = $1")
                    .bind(product_id)
                    .fetch_optional(&self.db.pool)
                    .await
                    .map_err(AppError::Database)?;

                if let Some(row) = product {
                    if let Ok(name) = row.try_get("name") {
                        receipt_items.push(ReceiptItem {
                            name,
                            quantity: item.quantity,
                            unit_price: item.unit_price,
                            discount: item.discount,
                            subtotal: item.subtotal,
                        });
                    }
                }
            }
        }

        Ok(ReceiptData {
            invoice_no: sale.invoice_no,
            customer_name,
            cashier_name,
            items: receipt_items,
            subtotal: sale.subtotal,
            tax_total: sale.tax_total,
            discount_total: sale.discount_total,
            grand_total: sale.grand_total,
            amount_paid: sale.amount_paid,
            change_given: sale.change_given,
            payment_method: sale.payment_method.as_str().to_string(),
            created_at: sale.created_at,
        })
    }

    // ==================== Cash Drawer Management ====================

    /// Open a new cash drawer session
    pub async fn open_cash_drawer(
        &self,
        request: CreateCashDrawerSessionRequest,
        user_id: i32,
    ) -> AppResult<CashDrawerSession> {
        let session = sqlx::query_as::<_, CashDrawerSession>(
            r#"
            INSERT INTO cash_drawer_sessions (user_id, opening_cash, closing_cash_expected, 
                                              closing_cash_actual, discrepancy, opened_at, closed_at)
            VALUES ($1, $2, $2, $2, 0, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
            RETURNING *
            "#,
        )
        .bind(user_id)
        .bind(request.opening_cash)
        .fetch_one(&self.db.pool)
        .await?;

        tracing::info!(
            "Cash drawer opened by user {}: ${}",
            user_id,
            request.opening_cash
        );
        Ok(session)
    }

    /// Close a cash drawer session
    pub async fn close_cash_drawer(
        &self,
        session_id: i32,
        request: CloseCashDrawerSessionRequest,
    ) -> AppResult<CashDrawerSession> {
        let mut transaction = self.db.begin().await?;

        // Get session with row lock
        let session = sqlx::query_as::<_, CashDrawerSession>(
            "SELECT * FROM cash_drawer_sessions WHERE id = $1 FOR UPDATE",
        )
        .bind(session_id)
        .fetch_optional(&mut *transaction)
        .await?
        .ok_or_else(|| {
            AppError::NotFound(format!("Cash drawer session {} not found", session_id))
        })?;

        // Calculate total cash sales for this session
        let cash_sales = sqlx::query_scalar::<_, Decimal>(
            r#"
            SELECT COALESCE(SUM(amount_paid), 0)
            FROM sales
            WHERE cashier_id = $1
            AND payment_method = 'cash'
            AND created_at >= $2
            AND created_at <= CURRENT_TIMESTAMP
            "#,
        )
        .bind(session.user_id)
        .bind(session.opened_at)
        .fetch_one(&mut *transaction)
        .await?;

        let expected_closing = session.opening_cash + cash_sales;
        let discrepancy = request.closing_cash_actual - expected_closing;

        // Update session
        let closed_session = sqlx::query_as::<_, CashDrawerSession>(
            r#"
            UPDATE cash_drawer_sessions
            SET closing_cash_expected = $1,
                closing_cash_actual = $2,
                discrepancy = $3,
                notes = $4,
                closed_at = CURRENT_TIMESTAMP
            WHERE id = $5
            RETURNING *
            "#,
        )
        .bind(expected_closing)
        .bind(request.closing_cash_actual)
        .bind(discrepancy)
        .bind(request.notes)
        .bind(session_id)
        .fetch_one(&mut *transaction)
        .await?;

        transaction.commit().await?;

        tracing::info!(
            "Cash drawer closed: session {}, expected: ${}, actual: ${}, discrepancy: ${}",
            session_id,
            expected_closing,
            request.closing_cash_actual,
            discrepancy
        );

        Ok(closed_session)
    }

    /// Get cash drawer sessions for a user
    pub async fn get_cash_drawer_sessions(
        &self,
        user_id: i32,
    ) -> AppResult<Vec<CashDrawerSession>> {
        sqlx::query_as::<_, CashDrawerSession>(
            "SELECT * FROM cash_drawer_sessions WHERE user_id = $1 ORDER BY opened_at DESC",
        )
        .bind(user_id)
        .fetch_all(&self.db.pool)
        .await
        .map_err(AppError::Database)
    }
}
