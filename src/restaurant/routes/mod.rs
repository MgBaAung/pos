use crate::database::Database;
use crate::restaurant::domain::kitchen_orders::KitchenOrderEvent;
use crate::restaurant::handlers::*;
use axum::{
    routing::{delete, get, post, put},
    Router,
};
use tokio::sync::broadcast;

#[derive(Clone)]
pub struct RestaurantAppState {
    pub database: Database,
    pub kitchen_event_sender: broadcast::Sender<KitchenOrderEvent>,
}

pub fn create_restaurant_router(
    database: Database,
    kitchen_event_sender: broadcast::Sender<KitchenOrderEvent>,
) -> Router {
    let app_state = RestaurantAppState {
        database,
        kitchen_event_sender,
    };

    // Table Management Routes
    let table_routes = Router::new()
        .route("/", post(create_table))
        .route("/", get(list_tables))
        .route("/available", get(get_available_tables))
        .route("/:id", get(get_table))
        .route("/:id", put(update_table))
        .route("/:id", delete(delete_table))
        .route("/:id/status", put(update_table_status))
        .route("/number/:table_number", get(get_table_by_number))
        .with_state(app_state.clone());

    // Modifier Management Routes
    let modifier_routes = Router::new()
        .route("/groups", post(create_modifier_group))
        .route("/groups", get(list_modifier_groups))
        .route("/groups/:id", get(get_modifier_group))
        .route("/groups/:id", put(update_modifier_group))
        .route("/groups/:id", delete(delete_modifier_group))
        .route("/", post(create_modifier))
        .route("/:id", get(get_modifier))
        .route("/:id", put(update_modifier))
        .route("/:id", delete(delete_modifier))
        .route(
            "/products/:product_id/groups",
            get(get_product_modifier_groups),
        )
        .route(
            "/products/groups/link",
            post(link_product_to_modifier_group),
        )
        .route(
            "/products/:product_id/groups/:modifier_group_id",
            delete(unlink_product_from_modifier_group),
        )
        .with_state(app_state.clone());

    // Kitchen Order Routes
    let kitchen_order_routes = Router::new()
        .route("/orders", post(create_kitchen_order))
        .route("/orders", get(list_kitchen_orders))
        .route("/orders/pending", get(get_pending_kitchen_orders))
        .route("/orders/active", get(get_active_kitchen_orders))
        .route("/orders/:id", get(get_kitchen_order))
        .route("/orders/:id", put(update_kitchen_order))
        .route("/orders/items/:id", put(update_kitchen_order_item))
        .with_state(app_state.clone());

    // Combine all restaurant routes
    Router::new()
        .nest("/tables", table_routes)
        .nest("/modifiers", modifier_routes)
        .nest("/kitchen", kitchen_order_routes)
}
