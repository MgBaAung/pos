use crate::database::Database;
use crate::error::{AppError, AppResult};
use chrono::{Datelike, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::Row;

#[derive(Debug, Serialize, Deserialize)]
pub struct DailySalesReport {
    pub date: NaiveDate,
    pub total_sales: i64,
    pub total_revenue: Decimal,
    pub total_tax: Decimal,
    pub total_discount: Decimal,
    pub cash_sales: Decimal,
    pub card_sales: Decimal,
    pub mobile_qr_sales: Decimal,
    pub average_order_value: Decimal,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MonthlySalesReport {
    pub year: i32,
    pub month: i32,
    pub total_sales: i64,
    pub total_revenue: Decimal,
    pub total_tax: Decimal,
    pub total_discount: Decimal,
    pub cash_sales: Decimal,
    pub card_sales: Decimal,
    pub mobile_qr_sales: Decimal,
    pub average_order_value: Decimal,
    pub daily_breakdown: Vec<DailySalesReport>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TopSellingProduct {
    pub product_id: i32,
    pub product_name: String,
    pub sku: String,
    pub total_quantity_sold: i64,
    pub total_revenue: Decimal,
    pub total_profit: Decimal,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CashierPerformance {
    pub cashier_id: i32,
    pub cashier_name: String,
    pub total_sales: i64,
    pub total_revenue: Decimal,
    pub average_order_value: Decimal,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RevenueSummary {
    pub total_revenue: Decimal,
    pub total_cost: Decimal,
    pub gross_profit: Decimal,
    pub profit_margin: Decimal,
    pub total_tax_collected: Decimal,
    pub total_discounts_given: Decimal,
}

pub struct ReportService {
    db: Database,
}

impl ReportService {
    pub fn new(db: Database) -> Self {
        ReportService { db }
    }

    // ==================== Daily Sales Report ====================

    /// Generate daily sales report for a specific date
    pub async fn daily_sales_report(&self, date: NaiveDate) -> AppResult<DailySalesReport> {
        let start_of_day = date.and_hms_opt(0, 0, 0).unwrap();
        let end_of_day = date.and_hms_opt(23, 59, 59).unwrap();

        let row = sqlx::query(
            r#"
            SELECT 
                COUNT(*) as total_sales,
                COALESCE(SUM(grand_total), 0) as total_revenue,
                COALESCE(SUM(tax_total), 0) as total_tax,
                COALESCE(SUM(discount_total), 0) as total_discount,
                COALESCE(SUM(CASE WHEN payment_method = 'cash' THEN grand_total ELSE 0 END), 0) as cash_sales,
                COALESCE(SUM(CASE WHEN payment_method = 'card' THEN grand_total ELSE 0 END), 0) as card_sales,
                COALESCE(SUM(CASE WHEN payment_method = 'mobile_qr' THEN grand_total ELSE 0 END), 0) as mobile_qr_sales,
                COALESCE(AVG(grand_total), 0) as average_order_value
            FROM sales
            WHERE created_at >= $1 AND created_at <= $2
            AND status = 'completed'
            "#,
        )
        .bind(start_of_day)
        .bind(end_of_day)
        .fetch_one(&self.db.pool)
        .await?;

        Ok(DailySalesReport {
            date,
            total_sales: row.get("total_sales"),
            total_revenue: row.get("total_revenue"),
            total_tax: row.get("total_tax"),
            total_discount: row.get("total_discount"),
            cash_sales: row.get("cash_sales"),
            card_sales: row.get("card_sales"),
            mobile_qr_sales: row.get("mobile_qr_sales"),
            average_order_value: row.get("average_order_value"),
        })
    }

    /// Generate daily sales report for today
    pub async fn today_sales_report(&self) -> AppResult<DailySalesReport> {
        let today = Utc::now().naive_utc().date();
        self.daily_sales_report(today).await
    }

    // ==================== Monthly Sales Report ====================

    /// Generate monthly sales report for a specific month
    pub async fn monthly_sales_report(
        &self,
        year: i32,
        month: i32,
    ) -> AppResult<MonthlySalesReport> {
        let start_of_month = NaiveDate::from_ymd_opt(year, month as u32, 1)
            .ok_or_else(|| AppError::Validation("Invalid date".to_string()))?
            .and_hms_opt(0, 0, 0)
            .unwrap();

        let end_of_month = if month == 12 {
            NaiveDate::from_ymd_opt(year + 1, 1, 1)
        } else {
            NaiveDate::from_ymd_opt(year, (month + 1) as u32, 1)
        }
        .ok_or_else(|| AppError::Validation("Invalid date".to_string()))?
        .and_hms_opt(0, 0, 0)
        .unwrap();

        let row = sqlx::query(
            r#"
            SELECT 
                COUNT(*) as total_sales,
                COALESCE(SUM(grand_total), 0) as total_revenue,
                COALESCE(SUM(tax_total), 0) as total_tax,
                COALESCE(SUM(discount_total), 0) as total_discount,
                COALESCE(SUM(CASE WHEN payment_method = 'cash' THEN grand_total ELSE 0 END), 0) as cash_sales,
                COALESCE(SUM(CASE WHEN payment_method = 'card' THEN grand_total ELSE 0 END), 0) as card_sales,
                COALESCE(SUM(CASE WHEN payment_method = 'mobile_qr' THEN grand_total ELSE 0 END), 0) as mobile_qr_sales,
                COALESCE(AVG(grand_total), 0) as average_order_value
            FROM sales
            WHERE created_at >= $1 AND created_at < $2
            AND status = 'completed'
            "#,
        )
        .bind(start_of_month)
        .bind(end_of_month)
        .fetch_one(&self.db.pool)
        .await?;

        // Get daily breakdown
        let daily_breakdown = sqlx::query(
            r#"
            SELECT 
                DATE(created_at) as date,
                COUNT(*) as total_sales,
                COALESCE(SUM(grand_total), 0) as total_revenue,
                COALESCE(SUM(tax_total), 0) as total_tax,
                COALESCE(SUM(discount_total), 0) as total_discount,
                COALESCE(SUM(CASE WHEN payment_method = 'cash' THEN grand_total ELSE 0 END), 0) as cash_sales,
                COALESCE(SUM(CASE WHEN payment_method = 'card' THEN grand_total ELSE 0 END), 0) as card_sales,
                COALESCE(SUM(CASE WHEN payment_method = 'mobile_qr' THEN grand_total ELSE 0 END), 0) as mobile_qr_sales,
                COALESCE(AVG(grand_total), 0) as average_order_value
            FROM sales
            WHERE created_at >= $1 AND created_at < $2
            AND status = 'completed'
            GROUP BY DATE(created_at)
            ORDER BY date
            "#,
        )
        .bind(start_of_month)
        .bind(end_of_month)
        .fetch_all(&self.db.pool)
        .await?;

        let daily_reports: Vec<DailySalesReport> = daily_breakdown
            .iter()
            .map(|row| DailySalesReport {
                date: row.get("date"),
                total_sales: row.get("total_sales"),
                total_revenue: row.get("total_revenue"),
                total_tax: row.get("total_tax"),
                total_discount: row.get("total_discount"),
                cash_sales: row.get("cash_sales"),
                card_sales: row.get("card_sales"),
                mobile_qr_sales: row.get("mobile_qr_sales"),
                average_order_value: row.get("average_order_value"),
            })
            .collect();

        Ok(MonthlySalesReport {
            year,
            month,
            total_sales: row.get("total_sales"),
            total_revenue: row.get("total_revenue"),
            total_tax: row.get("total_tax"),
            total_discount: row.get("total_discount"),
            cash_sales: row.get("cash_sales"),
            card_sales: row.get("card_sales"),
            mobile_qr_sales: row.get("mobile_qr_sales"),
            average_order_value: row.get("average_order_value"),
            daily_breakdown: daily_reports,
        })
    }

    /// Generate monthly sales report for current month
    pub async fn current_month_report(&self) -> AppResult<MonthlySalesReport> {
        let now = Utc::now().naive_utc();
        self.monthly_sales_report(now.year(), now.month() as i32)
            .await
    }

    // ==================== Top Selling Products ====================

    /// Get top selling products for a date range
    pub async fn top_selling_products(
        &self,
        start_date: NaiveDate,
        end_date: NaiveDate,
        limit: i64,
    ) -> AppResult<Vec<TopSellingProduct>> {
        let start_datetime = start_date.and_hms_opt(0, 0, 0).unwrap();
        let end_datetime = end_date.and_hms_opt(23, 59, 59).unwrap();

        let rows = sqlx::query(
            r#"
            SELECT 
                p.id as product_id,
                p.name as product_name,
                p.sku,
                SUM(si.quantity) as total_quantity_sold,
                SUM(si.subtotal) as total_revenue,
                SUM((si.unit_price - si.cost_price) * si.quantity) as total_profit
            FROM sale_items si
            JOIN sales s ON si.sale_id = s.id
            JOIN products p ON si.product_id = p.id
            WHERE s.created_at >= $1 AND s.created_at <= $2
            AND s.status = 'completed'
            GROUP BY p.id, p.name, p.sku
            ORDER BY total_quantity_sold DESC
            LIMIT $3
            "#,
        )
        .bind(start_datetime)
        .bind(end_datetime)
        .bind(limit)
        .fetch_all(&self.db.pool)
        .await?;

        let products: Vec<TopSellingProduct> = rows
            .iter()
            .map(|row| TopSellingProduct {
                product_id: row.get("product_id"),
                product_name: row.get("product_name"),
                sku: row.get("sku"),
                total_quantity_sold: row.get("total_quantity_sold"),
                total_revenue: row.get("total_revenue"),
                total_profit: row.get("total_profit"),
            })
            .collect();

        Ok(products)
    }

    /// Get top selling products for all time
    pub async fn all_time_top_selling_products(
        &self,
        limit: i64,
    ) -> AppResult<Vec<TopSellingProduct>> {
        let rows = sqlx::query(
            r#"
            SELECT 
                p.id as product_id,
                p.name as product_name,
                p.sku,
                SUM(si.quantity) as total_quantity_sold,
                SUM(si.subtotal) as total_revenue,
                SUM((si.unit_price - si.cost_price) * si.quantity) as total_profit
            FROM sale_items si
            JOIN sales s ON si.sale_id = s.id
            JOIN products p ON si.product_id = p.id
            WHERE s.status = 'completed'
            GROUP BY p.id, p.name, p.sku
            ORDER BY total_quantity_sold DESC
            LIMIT $1
            "#,
        )
        .bind(limit)
        .fetch_all(&self.db.pool)
        .await?;

        let products: Vec<TopSellingProduct> = rows
            .iter()
            .map(|row| TopSellingProduct {
                product_id: row.get("product_id"),
                product_name: row.get("product_name"),
                sku: row.get("sku"),
                total_quantity_sold: row.get("total_quantity_sold"),
                total_revenue: row.get("total_revenue"),
                total_profit: row.get("total_profit"),
            })
            .collect();

        Ok(products)
    }

    // ==================== Cashier Performance ====================

    /// Get cashier performance report for a date range
    pub async fn cashier_performance(
        &self,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> AppResult<Vec<CashierPerformance>> {
        let start_datetime = start_date.and_hms_opt(0, 0, 0).unwrap();
        let end_datetime = end_date.and_hms_opt(23, 59, 59).unwrap();

        let rows = sqlx::query(
            r#"
            SELECT 
                u.id as cashier_id,
                u.full_name as cashier_name,
                COUNT(s.id) as total_sales,
                COALESCE(SUM(s.grand_total), 0) as total_revenue,
                COALESCE(AVG(s.grand_total), 0) as average_order_value
            FROM sales s
            JOIN users u ON s.cashier_id = u.id
            WHERE s.created_at >= $1 AND s.created_at <= $2
            AND s.status = 'completed'
            GROUP BY u.id, u.full_name
            ORDER BY total_revenue DESC
            "#,
        )
        .bind(start_datetime)
        .bind(end_datetime)
        .fetch_all(&self.db.pool)
        .await?;

        let performance: Vec<CashierPerformance> = rows
            .iter()
            .map(|row| CashierPerformance {
                cashier_id: row.get("cashier_id"),
                cashier_name: row.get("cashier_name"),
                total_sales: row.get("total_sales"),
                total_revenue: row.get("total_revenue"),
                average_order_value: row.get("average_order_value"),
            })
            .collect();

        Ok(performance)
    }

    // ==================== Revenue Summary ====================

    /// Get revenue summary for a date range
    pub async fn revenue_summary(
        &self,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> AppResult<RevenueSummary> {
        let start_datetime = start_date.and_hms_opt(0, 0, 0).unwrap();
        let end_datetime = end_date.and_hms_opt(23, 59, 59).unwrap();

        let row = sqlx::query(
            r#"
            SELECT 
                COALESCE(SUM(s.grand_total), 0) as total_revenue,
                COALESCE(SUM(si.cost_price * si.quantity), 0) as total_cost,
                COALESCE(SUM(s.tax_total), 0) as total_tax_collected,
                COALESCE(SUM(s.discount_total), 0) as total_discounts_given
            FROM sales s
            JOIN sale_items si ON s.id = si.sale_id
            WHERE s.created_at >= $1 AND s.created_at <= $2
            AND s.status = 'completed'
            "#,
        )
        .bind(start_datetime)
        .bind(end_datetime)
        .fetch_one(&self.db.pool)
        .await?;

        let total_revenue: Decimal = row.get("total_revenue");
        let total_cost: Decimal = row.get("total_cost");
        let gross_profit = total_revenue - total_cost;
        let profit_margin = if total_revenue > Decimal::ZERO {
            (gross_profit / total_revenue) * Decimal::from(100)
        } else {
            Decimal::ZERO
        };

        Ok(RevenueSummary {
            total_revenue,
            total_cost,
            gross_profit,
            profit_margin,
            total_tax_collected: row.get("total_tax_collected"),
            total_discounts_given: row.get("total_discounts_given"),
        })
    }

    /// Get low stock alert report
    pub async fn low_stock_report(&self) -> AppResult<Vec<crate::models::Product>> {
        sqlx::query_as::<_, crate::models::Product>(
            r#"
            SELECT * FROM products
            WHERE stock_quantity <= low_stock_threshold
            AND is_active = true
            ORDER BY (stock_quantity::float / low_stock_threshold::float) ASC
            "#,
        )
        .fetch_all(&self.db.pool)
        .await
        .map_err(AppError::Database)
    }

    /// Get sales trend data for charting
    pub async fn sales_trend(
        &self,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> AppResult<Vec<(NaiveDate, Decimal)>> {
        let start_datetime = start_date.and_hms_opt(0, 0, 0).unwrap();
        let end_datetime = end_date.and_hms_opt(23, 59, 59).unwrap();

        let rows = sqlx::query(
            r#"
            SELECT 
                DATE(created_at) as date,
                COALESCE(SUM(grand_total), 0) as total_revenue
            FROM sales
            WHERE created_at >= $1 AND created_at <= $2
            AND status = 'completed'
            GROUP BY DATE(created_at)
            ORDER BY date
            "#,
        )
        .bind(start_datetime)
        .bind(end_datetime)
        .fetch_all(&self.db.pool)
        .await?;

        let trend: Vec<(NaiveDate, Decimal)> = rows
            .iter()
            .map(|row| (row.get("date"), row.get("total_revenue")))
            .collect();

        Ok(trend)
    }
}
