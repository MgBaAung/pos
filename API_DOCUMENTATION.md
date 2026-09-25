# POS System API Documentation

## Base URL
```
http://localhost:8080
```

## Authentication
Most endpoints require authentication using JWT tokens. Include the token in the Authorization header:
```
Authorization: Bearer <your-jwt-token>
```

## Response Format
All responses follow this structure:
```json
{
  "data": { ... },
  "error": null
}
```

Error responses:
```json
{
  "error": "Error message",
  "status": 400
}
```

## Endpoints

### Health Check
**GET** `/health`
- No authentication required
- Returns server health status

### Authentication

#### Register User
**POST** `/api/auth/register`
- No authentication required
- Body:
```json
{
  "username": "string",
  "password": "string",
  "full_name": "string",
  "role": "admin|store_manager|cashier"
}
```
- Response: User with JWT token

#### Login
**POST** `/api/auth/login`
- No authentication required
- Body:
```json
{
  "username": "string",
  "password": "string"
}
```
- Response: User with JWT token

#### Get Current User
**GET** `/api/auth/me`
- Authentication required
- Response: Current user information

### Products

#### List Products
**GET** `/api/products`
- No authentication required
- Query params:
  - `category_id` (optional): Filter by category
  - `active_only` (optional): Show only active products
  - `low_stock_only` (optional): Show only low stock products
- Response: Array of products

#### Get Product by ID
**GET** `/api/products/:id`
- No authentication required
- Response: Single product

#### Get Product by SKU
**GET** `/api/products/sku/:sku`
- No authentication required
- Response: Single product

#### Get Product by Barcode
**GET** `/api/products/barcode/:barcode`
- No authentication required
- Response: Single product

#### Create Product
**POST** `/api/products`
- Authentication required (Manager or Admin)
- Body:
```json
{
  "sku": "string",
  "barcode": "string (optional)",
  "name": "string",
  "description": "string (optional)",
  "category_id": "integer (optional)",
  "cost_price": "decimal",
  "selling_price": "decimal",
  "stock_quantity": "integer",
  "low_stock_threshold": "integer",
  "tax_rate": "decimal"
}
```
- Response: Created product

#### Update Product
**PUT** `/api/products/:id`
- Authentication required (Manager or Admin)
- Body: Same as create product (all fields optional)
- Response: Updated product

#### Delete Product
**DELETE** `/api/products/:id`
- Authentication required (Admin only)
- Response: Success message

#### Adjust Stock
**POST** `/api/products/:id/stock`
- Authentication required (Manager or Admin)
- Body:
```json
{
  "change_amount": "integer",
  "reason": "string"
}
```
- Response: Updated product

#### Get Low Stock Products
**GET** `/api/products/low-stock`
- No authentication required
- Response: Array of low stock products

### Categories

#### List Categories
**GET** `/api/categories`
- No authentication required
- Response: Array of categories

#### Get Category by ID
**GET** `/api/categories/:id`
- No authentication required
- Response: Single category

#### Create Category
**POST** `/api/categories`
- Authentication required (Manager or Admin)
- Body:
```json
{
  "name": "string",
  "description": "string (optional)",
  "parent_id": "integer (optional)"
}
```
- Response: Created category

#### Update Category
**PUT** `/api/categories/:id`
- Authentication required (Manager or Admin)
- Body: Same as create category (all fields optional)
- Response: Updated category

#### Delete Category
**DELETE** `/api/categories/:id`
- Authentication required (Admin only)
- Response: Success message

### Customers

#### List Customers
**GET** `/api/customers`
- No authentication required
- Query params:
  - `limit` (optional): Number of results (default: 50)
  - `offset` (optional): Offset for pagination (default: 0)
- Response: Array of customers

#### Get Customer by ID
**GET** `/api/customers/:id`
- No authentication required
- Response: Single customer

#### Get Customer by Phone
**GET** `/api/customers/phone/:phone`
- No authentication required
- Response: Single customer

#### Search Customers
**GET** `/api/customers/search/:query`
- No authentication required
- Response: Array of matching customers

#### Create Customer
**POST** `/api/customers`
- Authentication required (Cashier or higher)
- Body:
```json
{
  "name": "string",
  "phone": "string (optional)",
  "email": "string (optional)",
  "address": "string (optional)"
}
```
- Response: Created customer

#### Update Customer
**PUT** `/api/customers/:id`
- Authentication required (Cashier or higher)
- Body: Same as create customer (all fields optional)
- Response: Updated customer

#### Delete Customer
**DELETE** `/api/customers/:id`
- Authentication required (Cashier or higher)
- Response: Success message

#### Get Customer Purchase History
**GET** `/api/customers/:id/history`
- Authentication required (Cashier or higher)
- Response: Array of sales

### Sales

#### List Sales
**GET** `/api/sales`
- No authentication required
- Query params:
  - `cashier_id` (optional): Filter by cashier
  - `customer_id` (optional): Filter by customer
  - `status` (optional): Filter by status
  - `payment_method` (optional): Filter by payment method
  - `limit` (optional): Number of results (default: 50)
  - `offset` (optional): Offset for pagination (default: 0)
- Response: Array of sales

#### Get Sale by ID
**GET** `/api/sales/:id`
- No authentication required
- Response: Sale with items

#### Get Sale by Invoice Number
**GET** `/api/sales/invoice/:invoice_no`
- No authentication required
- Response: Sale with items

#### Create Sale
**POST** `/api/sales`
- Authentication required (Cashier or higher)
- Body:
```json
{
  "customer_id": "integer (optional)",
  "payment_method": "cash|card|mobile_qr",
  "amount_paid": "decimal",
  "discount_total": "decimal (optional)",
  "items": [
    {
      "product_id": "integer",
      "quantity": "integer",
      "discount": "decimal (optional)"
    }
  ]
}
```
- Response: Created sale with items

#### Refund Sale
**POST** `/api/sales/:id/refund`
- Authentication required (Manager or Admin)
- Response: Refunded sale

#### Generate Receipt
**GET** `/api/sales/:id/receipt`
- No authentication required
- Response: Receipt data for printing

### Cash Drawer

#### Open Cash Drawer
**POST** `/api/cash-drawer/open`
- Authentication required (Cashier or higher)
- Body:
```json
{
  "opening_cash": "decimal"
}
```
- Response: Created cash drawer session

#### Close Cash Drawer
**POST** `/api/cash-drawer/:id/close`
- Authentication required (Cashier or higher)
- Body:
```json
{
  "closing_cash_actual": "decimal",
  "notes": "string (optional)"
}
```
- Response: Closed cash drawer session with reconciliation

#### Get Cash Drawer Sessions
**GET** `/api/cash-drawer/sessions`
- Authentication required (Cashier or higher)
- Response: Array of cash drawer sessions for current user

### Reports

#### Daily Sales Report
**GET** `/api/reports/daily/:date`
- Authentication required (Manager or higher)
- Response: Daily sales summary

#### Today's Sales Report
**GET** `/api/reports/today`
- Authentication required (Manager or higher)
- Response: Today's sales summary

#### Monthly Sales Report
**GET** `/api/reports/monthly/:year/:month`
- Authentication required (Manager or higher)
- Response: Monthly sales summary with daily breakdown

#### Current Month Report
**GET** `/api/reports/current-month`
- Authentication required (Manager or higher)
- Response: Current month sales summary

#### Top Selling Products
**GET** `/api/reports/top-products`
- Authentication required (Manager or higher)
- Query params:
  - `start_date` (optional): Start date for filter
  - `end_date` (optional): End date for filter
- Response: Array of top selling products

#### Low Stock Report
**GET** `/api/reports/low-stock`
- Authentication required (Manager or higher)
- Response: Array of low stock products

#### Revenue Summary
**GET** `/api/reports/revenue`
- Authentication required (Admin only)
- Query params:
  - `start_date` (optional): Start date for filter
  - `end_date` (optional): End date for filter
- Response: Revenue summary with profit calculations

#### Sales Trend
**GET** `/api/reports/sales-trend`
- Authentication required (Manager or higher)
- Query params:
  - `start_date` (optional): Start date for filter
  - `end_date` (optional): End date for filter
- Response: Array of date-revenue pairs for charting

## Role Permissions

### Admin
- Full access to all endpoints
- User management
- System configuration
- Financial reports

### Store Manager
- Product and category management
- Inventory adjustments
- Sales processing and refunds
- View reports (except financial)
- Cash drawer management

### Cashier
- Process sales
- View products and customers
- Cash drawer management
- Basic reports

## Error Codes

- `200` - Success
- `400` - Bad Request (validation error)
- `401` - Unauthorized (authentication required)
- `403` - Forbidden (insufficient permissions)
- `404` - Not Found
- `409` - Conflict (duplicate resource)
- `500` - Internal Server Error

## Data Types

### User Role
- `admin`
- `store_manager`
- `cashier`

### Payment Method
- `cash`
- `card`
- `mobile_qr`

### Sale Status
- `completed`
- `refunded`
- `cancelled`

### Adjustment Type
- `restock`
- `damage`
- `manual_correction`
- `sale`

## Date Format
All dates should be in ISO 8601 format: `YYYY-MM-DD`

## Decimal Format
All monetary values use decimal format with 2 decimal places.
