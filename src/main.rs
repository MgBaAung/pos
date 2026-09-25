mod auth;
mod config;
mod customers;
mod database;
mod error;
mod handlers;
mod inventory;
mod middleware;
mod models;
mod printer;
mod reports;
mod restaurant;
mod sales;
mod service;
mod utils;

use crate::service::BranchService;
use auth::JwtService;
use axum::{
    http::Method,
    routing::{delete, get, post, put},
    Router,
};
use config::init_config;
use database::Database;
use handlers::*;
use restaurant::{create_kitchen_broadcast_manager, routes::create_restaurant_router};
use std::sync::Arc;
use std::time::Duration;
use tower_http::compression::CompressionLayer;
use tower_http::cors::{Any, CorsLayer};
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::TraceLayer;
use utils::init_logging;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load configuration
    let settings = init_config()?;

    // Initialize logging
    init_logging(&settings);

    tracing::info!(
        "Starting {} v{}",
        settings.app.name,
        env!("CARGO_PKG_VERSION")
    );
    tracing::info!("Environment: {}", settings.app.environment);

    // Create database connection pool
    let database = Database::new(&settings).await?;

    // Run database migrations
    database.migrate().await?;

    // Shared handle for handlers that expect `Arc<Database>`
    let db = Arc::new(database.clone());

    // Initialize JWT service
    let jwt_service = Arc::new(JwtService::new(
        &settings.jwt.secret,
        settings.jwt.expiration_hours,
    ));

    // Initialize branch service
    let branch_service = Arc::new(BranchService::new(db.clone()));

    // Initialize kitchen broadcast manager for real-time updates
    let kitchen_broadcast_manager = create_kitchen_broadcast_manager();

    // Expose the kitchen event sender process-wide so the sales service can
    // auto-push kitchen orders after a dine-in sale commits.
    restaurant::realtime::init_global_kitchen_sender(kitchen_broadcast_manager.get_sender());

    // Configure CORS
    let allowed_origins = settings
        .cors
        .allowed_origins
        .iter()
        .filter_map(|origin| match origin.parse() {
            Ok(parsed) => Some(parsed),
            Err(e) => {
                tracing::warn!("Ignoring invalid CORS origin '{}': {}", origin, e);
                None
            }
        })
        .collect::<Vec<_>>();

    let cors = CorsLayer::new()
        .allow_origin(allowed_origins)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers(Any)
        .allow_credentials(true);

    // Public routes backed by `State<Database>` (no authentication required)
    let public_db_routes = Router::new()
        .route("/health", get(health_check))
        .route("/api/products", get(list_products))
        .route("/api/products/:id", get(get_product))
        .route("/api/products/sku/:sku", get(get_product_by_sku))
        .route(
            "/api/products/barcode/:barcode",
            get(get_product_by_barcode),
        )
        .route("/api/products/low-stock", get(get_low_stock_products))
        .route("/api/categories", get(list_categories))
        .route("/api/categories/:id", get(get_category))
        .route("/api/customers", get(list_customers))
        .route("/api/customers/:id", get(get_customer))
        .route("/api/customers/phone/:phone", get(get_customer_by_phone))
        .route("/api/customers/search/:query", get(search_customers))
        .route("/api/sales", get(list_sales))
        .route("/api/sales/:id", get(get_sale))
        .route("/api/sales/invoice/:invoice_no", get(get_sale_by_invoice))
        .route("/api/sales/:id/receipt", get(generate_receipt))
        .route(
            "/api/sales/:id/receipt/escpos",
            get(generate_escpos_receipt),
        )
        .with_state(database.clone());

    // Public auth routes backed by `State<(Arc<Database>, Arc<JwtService>)>`
    let public_auth_routes = Router::new()
        .route("/api/auth/register", post(register))
        .route("/api/auth/login", post(login))
        .with_state((db.clone(), jwt_service.clone()));

    // Protected routes backed by `State<Database>`
    let protected_db_routes = Router::new()
        .route("/api/products", post(create_product))
        .route("/api/products/:id", put(update_product))
        .route("/api/products/:id", delete(delete_product))
        .route("/api/products/:id/stock", post(adjust_stock))
        .route(
            "/api/products/:id/variants",
            get(list_product_variants).post(create_product_variant),
        )
        .route("/api/variants/:id", get(get_product_variant))
        .route("/api/variants/:id", put(update_product_variant))
        .route("/api/variants/:id", delete(delete_product_variant))
        .route("/api/categories", post(create_category))
        .route("/api/categories/:id", put(update_category))
        .route("/api/categories/:id", delete(delete_category))
        .route("/api/customers", post(create_customer))
        .route("/api/customers/:id", put(update_customer))
        .route("/api/customers/:id", delete(delete_customer))
        .route(
            "/api/customers/:id/history",
            get(get_customer_purchase_history),
        )
        .route("/api/sales", post(create_sale))
        .route("/api/sales/:id/refund", post(refund_sale))
        .route("/api/cash-drawer/open", post(open_cash_drawer))
        .route("/api/cash-drawer/:id/close", post(close_cash_drawer))
        .route("/api/cash-drawer/sessions", get(get_cash_drawer_sessions))
        .route("/api/reports/daily/:date", get(daily_sales_report))
        .route("/api/reports/today", get(today_sales_report))
        .route(
            "/api/reports/monthly/:year/:month",
            get(monthly_sales_report),
        )
        .route("/api/reports/current-month", get(current_month_report))
        .route("/api/reports/top-products", get(top_selling_products))
        .route("/api/reports/low-stock", get(low_stock_report))
        .route("/api/reports/revenue", get(revenue_summary))
        .route("/api/reports/sales-trend", get(sales_trend))
        .route("/api/customers/top", get(get_top_customers))
        .route(
            "/api/customers/loyalty",
            get(get_customers_by_loyalty_points),
        )
        .route("/api/customers/:id/loyalty/add", post(add_loyalty_points))
        .route(
            "/api/customers/:id/loyalty/redeem",
            post(redeem_loyalty_points),
        )
        .route(
            "/api/products/:id/inventory-logs",
            get(get_product_inventory_logs),
        )
        .route("/api/inventory-logs/recent", get(get_recent_inventory_logs))
        .route("/api/users", get(list_users))
        .route("/api/users", post(create_user))
        .route("/api/users/:id", get(get_user))
        .route("/api/users/:id", put(update_user))
        .route("/api/users/:id", delete(deactivate_user))
        .route("/api/users/:id/password", post(reset_user_password))
        .with_state(database.clone());

    // Protected auth routes backed by `State<(Arc<Database>, Arc<JwtService>)>`
    let protected_auth_routes = Router::new()
        .route("/api/auth/me", get(me))
        .route("/api/auth/switch-branch", post(switch_branch))
        .with_state((db.clone(), jwt_service.clone()));

    // Protected branch routes backed by
    // `State<(Arc<Database>, Arc<JwtService>, Arc<BranchService>)>`
    let protected_branch_routes = Router::new()
        .route("/api/branches", get(list_branches))
        .route("/api/branches", post(create_branch))
        .route("/api/branches/:id", get(get_branch))
        .route("/api/branches/code/:code", get(get_branch_by_code))
        .route("/api/branches/:id", put(update_branch))
        .route("/api/branches/:id", delete(delete_branch))
        .route("/api/branches/performance", get(get_branch_performance))
        .route("/api/branches/comparison", get(get_branch_comparison))
        .route(
            "/api/branches/inventory-summary",
            get(get_branch_inventory_summary),
        )
        .route("/api/branches/assign-user", post(assign_user_to_branch))
        .route(
            "/api/branches/users/:user_id/branches/:branch_id",
            delete(remove_user_from_branch),
        )
        .route(
            "/api/branches/users/:user_id/access",
            get(get_user_branch_access),
        )
        .route(
            "/api/branches/users/:user_id/assignments",
            get(get_user_branch_assignments),
        )
        .route("/api/branches/:id/users", get(get_branch_users))
        .with_state((db.clone(), jwt_service.clone(), branch_service.clone()));

    // Combine all protected routes and require authentication
    let protected_routes = Router::new()
        .merge(protected_db_routes)
        .merge(protected_auth_routes)
        .merge(protected_branch_routes)
        .route_layer(axum::middleware::from_fn_with_state(
            jwt_service.clone(),
            crate::middleware::auth::auth_middleware,
        ));

    // Restaurant module routes (including WebSocket)
    let restaurant_routes =
        create_restaurant_router(database.clone(), kitchen_broadcast_manager.get_sender());

    // WebSocket route for kitchen display
    let websocket_route = Router::new()
        .route(
            "/api/ws/kitchen",
            axum::routing::get(restaurant::kitchen_websocket_handler),
        )
        .with_state(kitchen_broadcast_manager);

    // HTTP routes get a request timeout; the long-lived kitchen WebSocket is
    // merged afterwards so it is not subject to that timeout.
    let http_routes = Router::new()
        .merge(public_db_routes)
        .merge(public_auth_routes)
        .merge(protected_routes)
        .nest("/api/restaurant", restaurant_routes)
        .layer(TimeoutLayer::new(Duration::from_secs(30)));

    // Combine all routes
    let app = Router::new()
        .merge(http_routes)
        .merge(websocket_route)
        .layer(cors)
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http());

    let listener = tokio::net::TcpListener::bind(&settings.server_address()).await?;
    tracing::info!("Server listening on {}", settings.server_address());

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    tracing::info!("Server shut down gracefully");

    Ok(())
}

/// Resolve when the process receives Ctrl-C or a SIGTERM.
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl-C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::info!("Shutdown signal received, stopping server...");
}
