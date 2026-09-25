# POS System - Rust Backend Project Structure

```
pos-system-rust/
├── Cargo.toml                          # Project dependencies and metadata
├── .env.example                        # Environment variables template
├── .gitignore
├── README.md
├── RESTAURANT_INTEGRATION_GUIDE.md     # Restaurant module integration guide
├── PROJECT_STRUCTURE.md                # This file
├── migrations/                         # SQL migration files
│   ├── 001_add_restaurant_tables.sql  # Restaurant features migration
│   ├── 001_initial_schema.up.sql
│   └── 001_initial_schema.down.sql
├── src/
│   ├── main.rs                         # Application entry point
│   ├── lib.rs                          # Library exports
│   ├── config.rs                       # Configuration management
│   ├── database.rs                     # Database connection and pool setup
│   ├── error.rs                        # Custom error types
│   ├── models/                         # Database models (Retail)
│   │   ├── mod.rs
│   │   ├── user.rs
│   │   ├── product.rs
│   │   ├── category.rs
│   │   ├── customer.rs
│   │   ├── sale.rs
│   │   └── inventory_log.rs
│   ├── handlers/                       # API route handlers (Retail)
│   │   ├── mod.rs
│   │   ├── auth.rs
│   │   ├── products.rs
│   │   ├── categories.rs
│   │   ├── customers.rs
│   │   ├── sales.rs
│   │   └── reports.rs
│   ├── middleware/                     # Middleware (auth, RBAC, logging)
│   │   ├── mod.rs
│   │   ├── auth.rs
│   │   └── rbac.rs
│   ├── auth/                           # Authentication logic
│   │   ├── mod.rs
│   │   ├── jwt.rs
│   │   └── password.rs
│   ├── inventory/                      # Inventory business logic
│   │   ├── mod.rs
│   │   └── service.rs
│   ├── sales/                          # Sales/checkout engine
│   │   ├── mod.rs
│   │   ├── cart.rs
│   │   └── service.rs
│   ├── customers/                      # CRM logic
│   │   ├── mod.rs
│   │   └── service.rs
│   ├── reports/                        # Reporting/analytics
│   │   ├── mod.rs
│   │   └── service.rs
│   ├── restaurant/                     # Restaurant & F&B Module
│   │   ├── domain/                     # Domain models
│   │   │   ├── mod.rs
│   │   │   ├── tables.rs               # Table management models
│   │   │   ├── modifiers.rs            # Product modifiers & variants
│   │   │   └── kitchen_orders.rs       # Kitchen Order Tickets (KOT) models
│   │   ├── services/                   # Business logic layer
│   │   │   ├── mod.rs
│   │   │   ├── table_service.rs        # Table management service
│   │   │   ├── modifier_service.rs     # Modifier management service
│   │   │   └── kitchen_order_service.rs # KOT/KDS service
│   │   ├── handlers/                   # HTTP request handlers
│   │   │   ├── mod.rs
│   │   │   ├── table_handlers.rs
│   │   │   ├── modifier_handlers.rs
│   │   │   └── kitchen_order_handlers.rs
│   │   ├── routes/                     # Route definitions
│   │   │   └── mod.rs
│   │   ├── realtime/                   # WebSocket & real-time
│   │   │   └── mod.rs                  # Kitchen display WebSocket handler
│   │   └── mod.rs                      # Module exports
│   └── utils/                          # Utility functions
│       ├── mod.rs
│       └── validation.rs
└── tests/                              # Integration tests
    └── api_tests.rs
```

## Technology Stack

- **Language**: Rust 1.75+
- **Async Runtime**: Tokio
- **Web Framework**: Axum (high-performance async web framework)
- **Database**: PostgreSQL with sqlx (compile-time checked queries)
- **Authentication**: JWT (jsonwebtoken) + Argon2 (password hashing)
- **Serialization**: Serde
- **Validation**: validator
- **Error Handling**: thiserror + anyhow
- **Logging**: tracing + tracing-subscriber
- **Configuration**: config + dotenvy

## Key Design Decisions

1. **Axum over Actix-web**: Axum is built on Tokio and Tower, providing excellent async performance and a more ergonomic API for building REST APIs.

2. **sqlx with compile-time verification**: Prevents SQL errors at compile time and provides type-safe database queries.

3. **Modular architecture**: Clear separation of concerns with dedicated modules for each business domain.

4. **Domain-Driven Design**: Restaurant module follows DDD principles with clear separation of domain models, services, and handlers.

5. **Transaction safety**: All sales operations wrapped in database transactions to prevent race conditions.

6. **Structured logging**: Using tracing for distributed tracing and structured logs.

7. **Connection pooling**: Optimized for high concurrent POS terminals.

8. **Real-time communication**: WebSocket support for live kitchen display updates using tokio::sync::broadcast.

9. **Strict module separation**: Restaurant module is completely isolated from retail logic, ensuring zero pollution of existing code.
