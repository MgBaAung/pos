# POS System - Production-Ready Rust Backend

A high-performance, production-ready Point of Sale (POS) backend system built with Rust, designed for small businesses (Retail/F&B) with comprehensive inventory management, sales processing, and reporting capabilities.

## 🚀 Features

### Authentication & RBAC
- **JWT-based authentication** with secure token generation and validation
- **Argon2 password hashing** for secure password storage
- **Role-Based Access Control (RBAC)** with three roles:
  - **Admin**: Full system access
  - **Store Manager**: Inventory and sales management
  - **Cashier**: Sales processing and basic operations
- **Granular permissions** for each endpoint based on user roles

### Inventory & Product Management
- **Product CRUD operations** with SKU and barcode support
- **Category management** with hierarchical sub-categories
- **Real-time stock tracking** with automatic low-stock alerts
- **Stock adjustments** (restock, damage, manual correction) with audit logs
- **Barcode scanner integration** for quick product lookup
- **Transaction-safe stock updates** using PostgreSQL row-level locking

### Sales & Checkout Engine
- **Shopping cart management** with add, update, and remove operations
- **Item-level and cart-level discounts** (percentage or fixed amount)
- **Tax calculation engine** with per-product tax rates
- **Multiple payment methods**: Cash, Credit/Debit Card, Mobile/QR Payment
- **ACID transaction processing** to prevent race conditions and overselling
- **Receipt generation** with complete transaction details
- **Sale refund functionality** with automatic stock restoration

### Customer Management (CRM)
- **Customer profiles** with contact information
- **Loyalty points system** with accumulation and redemption
- **Purchase history tracking** per customer
- **Customer search** by name, phone, or email

### Reporting & Analytics
- **Daily sales reports** with payment method breakdown
- **Monthly sales reports** with daily breakdown
- **Top-selling products** analysis
- **Cash drawer reconciliation** (Z-Report)
- **Revenue summaries** with profit calculations
- **Sales trend data** for charting
- **Low stock alerts** for inventory management

### Restaurant & F&B Workflow
- **Table management** with status tracking (available, occupied, reserved, cleaning)
- **Product modifiers & variants** (meat choices, spice levels, add-ons)
- **Kitchen Order Tickets (KOT)** with item-level status tracking
- **Kitchen Display System (KDS)** with real-time WebSocket updates
- **Order type support** (dine-in, takeaway, delivery)
- **Zone-based table organization** (Indoor, Outdoor, VIP)
- **Priority-based kitchen routing**
- **Seamless integration** with existing retail POS features

### Technical Features
- **Highly concurrent** using Tokio async runtime
- **Connection pooling** optimized for multiple POS terminals
- **Structured logging** with tracing
- **Comprehensive error handling** with custom error types
- **Database migrations** with sqlx
- **CORS support** for frontend integration
- **Input validation** using validator crate
- **Modular architecture** with strict separation of concerns
- **Real-time WebSocket updates** for kitchen display systems

## 📋 Tech Stack

- **Language**: Rust 1.75+
- **Async Runtime**: Tokio
- **Web Framework**: Axum
- **Database**: PostgreSQL with sqlx (compile-time checked queries)
- **Authentication**: JWT (jsonwebtoken) + Argon2
- **Serialization**: Serde
- **Validation**: validator
- **Error Handling**: thiserror + anyhow
- **Logging**: tracing + tracing-subscriber
- **Configuration**: config + dotenvy
- **Decimal Handling**: rust_decimal (financial calculations)

## 🏗️ Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                         Frontend (TBD)                       │
└─────────────────────────────────────────────────────────────┘
                            │
                            │ HTTP/REST API
                            │
┌─────────────────────────────────────────────────────────────┐
│                     Backend (Axum)                          │
├─────────────────────────────────────────────────────────────┤
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │Auth Middleware│  │RBAC Middleware│  │Validation   │      │
│  └──────────────┘  └──────────────┘  └──────────────┘      │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │Inventory     │  │Sales Engine  │  │CRM Service   │      │
│  │Service       │  │              │  │              │      │
│  └──────────────┘  └──────────────┘  └──────────────┘      │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │Reporting     │  │JWT Service   │  │Password      │      │
│  │Service       │  │              │  │Service       │      │
│  └──────────────┘  └──────────────┘  └──────────────┘      │
└─────────────────────────────────────────────────────────────┘
                            │
                            │ SQL (Connection Pool)
                            │
┌─────────────────────────────────────────────────────────────┐
│                    Database (PostgreSQL)                     │
├─────────────────────────────────────────────────────────────┤
│  users | products | categories | sales | sale_items          │
│  customers | inventory_logs | cash_drawer_sessions           │
└─────────────────────────────────────────────────────────────┘
```

## 🗄️ Database Schema

### Core Tables
- **users**: User accounts with roles and authentication
- **categories**: Product categories with hierarchical support
- **products**: Product inventory with pricing and stock tracking
- **inventory_logs**: Audit trail for stock adjustments
- **customers**: Customer profiles with loyalty points
- **sales**: Sales transactions with payment details
- **sale_items**: Line items for each sale
- **cash_drawer_sessions**: Cash drawer reconciliation records

### Key Features
- **Foreign key constraints** for data integrity
- **Indexes** on frequently queried columns
- **Custom types** for enums (user_role, payment_method, etc.)
- **Timestamps** for audit trails

## 🚦 Getting Started

### Prerequisites
- Rust 1.75 or higher
- PostgreSQL 12 or higher
- (Optional) Docker for running PostgreSQL

### Installation

1. **Clone the repository**
```bash
git clone <repository-url>
cd pos-system-rust
```

2. **Set up environment variables**
```bash
cp .env.example .env
# Edit .env with your configuration
```

3. **Configure PostgreSQL**
```bash
# Create database
createdb pos_system

# Or using Docker
docker run --name postgres-pos \
  -e POSTGRES_PASSWORD=your_password \
  -e POSTGRES_DB=pos_system \
  -p 5432:5432 \
  -d postgres:15
```

4. **Install dependencies**
```bash
cargo build
```

5. **Run migrations**
```bash
# Migrations run automatically on startup
# Or run manually:
sqlx migrate run --database-url "postgresql://user:pass@localhost/pos_system"
```

6. **Start the server**
```bash
cargo run
```

The server will start on `http://localhost:8080` by default.

## 📡 API Endpoints

### Authentication
- `POST /api/auth/register` - Register new user
- `POST /api/auth/login` - Login user
- `GET /api/auth/me` - Get current user info (protected)

### Products
- `GET /api/products` - List products
- `GET /api/products/:id` - Get product by ID
- `GET /api/products/sku/:sku` - Get product by SKU
- `GET /api/products/barcode/:barcode` - Get product by barcode
- `POST /api/products` - Create product (protected)
- `PUT /api/products/:id` - Update product (protected)
- `DELETE /api/products/:id` - Delete product (protected)
- `POST /api/products/:id/stock` - Adjust stock (protected)
- `GET /api/products/low-stock` - Get low stock products

### Categories
- `GET /api/categories` - List categories
- `GET /api/categories/:id` - Get category by ID
- `POST /api/categories` - Create category (protected)
- `PUT /api/categories/:id` - Update category (protected)
- `DELETE /api/categories/:id` - Delete category (protected)

### Customers
- `GET /api/customers` - List customers
- `GET /api/customers/:id` - Get customer by ID
- `GET /api/customers/phone/:phone` - Get customer by phone
- `GET /api/customers/search/:query` - Search customers
- `POST /api/customers` - Create customer (protected)
- `PUT /api/customers/:id` - Update customer (protected)
- `DELETE /api/customers/:id` - Delete customer (protected)
- `GET /api/customers/:id/history` - Get purchase history (protected)

### Sales
- `GET /api/sales` - List sales
- `GET /api/sales/:id` - Get sale by ID
- `GET /api/sales/invoice/:invoice_no` - Get sale by invoice number
- `POST /api/sales` - Create sale (protected)
- `POST /api/sales/:id/refund` - Refund sale (protected)
- `GET /api/sales/:id/receipt` - Generate receipt

### Cash Drawer
- `POST /api/cash-drawer/open` - Open cash drawer (protected)
- `POST /api/cash-drawer/:id/close` - Close cash drawer (protected)
- `GET /api/cash-drawer/sessions` - Get cash drawer sessions (protected)

### Reports
- `GET /api/reports/daily/:date` - Daily sales report (protected)
- `GET /api/reports/today` - Today's sales report (protected)
- `GET /api/reports/monthly/:year/:month` - Monthly sales report (protected)
- `GET /api/reports/current-month` - Current month report (protected)
- `GET /api/reports/top-products` - Top selling products (protected)
- `GET /api/reports/low-stock` - Low stock report (protected)
- `GET /api/reports/revenue` - Revenue summary (protected)
- `GET /api/reports/sales-trend` - Sales trend data (protected)

### Health
- `GET /health` - Health check endpoint

### Restaurant Module (Base: `/api/restaurant`)

#### Table Management
- `POST /api/restaurant/tables` - Create table (protected)
- `GET /api/restaurant/tables` - List tables (protected)
- `GET /api/restaurant/tables/available` - Get available tables (protected)
- `GET /api/restaurant/tables/:id` - Get table by ID (protected)
- `PUT /api/restaurant/tables/:id` - Update table (protected)
- `PUT /api/restaurant/tables/:id/status` - Update table status (protected)
- `DELETE /api/restaurant/tables/:id` - Delete table (protected)

#### Modifiers
- `POST /api/restaurant/modifiers/groups` - Create modifier group (protected)
- `GET /api/restaurant/modifiers/groups` - List modifier groups (protected)
- `GET /api/restaurant/modifiers/groups/:id` - Get modifier group (protected)
- `PUT /api/restaurant/modifiers/groups/:id` - Update modifier group (protected)
- `DELETE /api/restaurant/modifiers/groups/:id` - Delete modifier group (protected)
- `POST /api/restaurant/modifiers` - Create modifier (protected)
- `GET /api/restaurant/modifiers/:id` - Get modifier (protected)
- `PUT /api/restaurant/modifiers/:id` - Update modifier (protected)
- `DELETE /api/restaurant/modifiers/:id` - Delete modifier (protected)
- `GET /api/restaurant/modifiers/products/:product_id/groups` - Get product modifier groups (protected)
- `POST /api/restaurant/modifiers/products/groups/link` - Link product to modifier group (protected)
- `DELETE /api/restaurant/modifiers/products/:product_id/groups/:modifier_group_id` - Unlink product from modifier group (protected)

#### Kitchen Orders (KOT/KDS)
- `POST /api/restaurant/kitchen/orders` - Create kitchen order (protected)
- `GET /api/restaurant/kitchen/orders` - List kitchen orders (protected)
- `GET /api/restaurant/kitchen/orders/pending` - Get pending orders (protected)
- `GET /api/restaurant/kitchen/orders/active` - Get active orders (protected)
- `GET /api/restaurant/kitchen/orders/:id` - Get kitchen order (protected)
- `PUT /api/restaurant/kitchen/orders/:id` - Update kitchen order (protected)
- `PUT /api/restaurant/kitchen/orders/items/:id` - Update kitchen order item (protected)

#### WebSocket
- `WS /api/ws/kitchen` - Kitchen display real-time updates (protected)

## 🔒 Security Features

1. **Password Hashing**: All passwords hashed with Argon2
2. **JWT Authentication**: Secure token-based authentication
3. **Role-Based Access Control**: Granular permissions per endpoint
4. **Input Validation**: Comprehensive validation using validator crate
5. **SQL Injection Prevention**: Parameterized queries via sqlx
6. **CORS Configuration**: Configured for secure frontend-backend communication
7. **Transaction Safety**: ACID transactions prevent race conditions

## 🧪 Testing

```bash
# Run tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test
cargo test test_name
```

## 📝 Configuration

Configuration is managed through environment variables. See `.env.example` for reference:

```env
DATABASE_URL=postgresql://postgres:password@localhost:5432/pos_system
DATABASE_MAX_CONNECTIONS=10
DATABASE_MIN_CONNECTIONS=5

SERVER_HOST=0.0.0.0
SERVER_PORT=8080

JWT_SECRET=your-super-secret-jwt-key-change-this-in-production
JWT_EXPIRATION_HOURS=24

CORS_ALLOWED_ORIGINS=http://localhost:3000,http://localhost:5173

RUST_LOG=debug
LOG_FORMAT=json
```

## 🚀 Deployment

### Using Docker
```bash
# Build image
docker build -t pos-system .

# Run container
docker run -p 8080:8080 \
  -e DATABASE_URL=postgresql://... \
  -e JWT_SECRET=... \
  pos-system
```

### Using Cargo
```bash
# Build release
cargo build --release

# Run
./target/release/pos-system
```

## 📊 Performance Considerations

- **Connection Pooling**: Optimized for high concurrent POS terminals
- **Database Indexes**: Strategic indexes on frequently queried columns
- **Async Operations**: Non-blocking I/O with Tokio
- **Transaction Safety**: Row-level locking prevents race conditions
- **Query Optimization**: Efficient SQL queries with proper joins

## 🤝 Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests
5. Submit a pull request

## 📄 License

This project is licensed under the MIT License.

## 🆘 Support

For issues and questions, please open an issue on the repository.

## 🗺️ Roadmap

- [x] Restaurant & F&B workflow module
- [x] Real-time WebSocket updates for kitchen display
- [ ] Frontend implementation (React/Vue)
- [ ] Advanced reporting with charts
- [ ] Multi-store support
- [ ] Email notifications
- [ ] Advanced inventory forecasting
- [ ] Integration with payment gateways
- [ ] Mobile app support
