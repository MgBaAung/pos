# Restaurant & F&B Module Integration Guide

## Overview

This guide documents the integration of the Restaurant and F&B workflow module into the existing Retail POS backend. The restaurant module is built using a strictly modular architecture (Domain-Driven Design) to ensure complete separation from the core retail logic.

## Architecture

### Modular Directory Structure

```
src/
├── restaurant/                          # Restaurant & F&B Module
│   ├── domain/                         # Domain Models
│   │   ├── mod.rs
│   │   ├── tables.rs                   # Table management models
│   │   ├── modifiers.rs                # Product modifiers & variants
│   │   └── kitchen_orders.rs           # Kitchen Order Tickets (KOT) models
│   ├── services/                       # Business Logic Layer
│   │   ├── mod.rs
│   │   ├── table_service.rs            # Table management service
│   │   ├── modifier_service.rs         # Modifier management service
│   │   └── kitchen_order_service.rs    # KOT/KDS service
│   ├── handlers/                       # HTTP Request Handlers
│   │   ├── mod.rs
│   │   ├── table_handlers.rs
│   │   ├── modifier_handlers.rs
│   │   └── kitchen_order_handlers.rs
│   ├── routes/                         # Route Definitions
│   │   └── mod.rs
│   ├── realtime/                       # WebSocket & Real-time
│   │   └── mod.rs                      # Kitchen display WebSocket handler
│   └── mod.rs                          # Module exports
├── auth/                               # Existing Retail Module
├── inventory/                          # Existing Retail Module
├── sales/                              # Existing Retail Module
├── customers/                          # Existing Retail Module
├── reports/                            # Existing Retail Module
└── main.rs                             # Application entry point
```

### Key Design Principles

1. **Strict Separation**: Restaurant module is completely isolated from retail logic
2. **Domain-Driven Design**: Clear separation of domain models, services, and handlers
3. **Clean Architecture**: Service layer handles business logic, handlers handle HTTP concerns
4. **Real-time Updates**: WebSocket integration for live kitchen display updates
5. **ACID Transactions**: All kitchen operations use database transactions for data integrity

## Database Schema

### New Tables

The restaurant module introduces the following database tables (see `migrations/001_add_restaurant_tables.sql`):

#### 1. Restaurant Tables
- `restaurant_tables`: Table management with status tracking
- Status: available, occupied, reserved, cleaning, maintenance
- Supports zones (Indoor, Outdoor, VIP, etc.)

#### 2. Product Modifiers
- `modifier_groups`: Categories of modifiers (e.g., "Meat Choice", "Spice Level")
- `modifiers`: Individual modifier options with price adjustments
- `product_modifier_groups`: Links products to modifier groups

#### 3. Kitchen Orders (KOT)
- `kitchen_orders`: Main kitchen order tickets
- `kitchen_order_items`: Item-level tracking with modifiers
- Status: pending, preparing, ready, served, cancelled, delayed

#### 4. Extended Sales Table
- Added `table_id`, `order_type`, `guest_count`, `special_requests` to existing sales table

## API Endpoints

### Base URL: `/api/restaurant`

### Table Management

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/api/restaurant/tables` | Create new table |
| GET | `/api/restaurant/tables` | List all tables (filterable by status, zone) |
| GET | `/api/restaurant/tables/available` | Get available tables |
| GET | `/api/restaurant/tables/:id` | Get table by ID |
| GET | `/api/restaurant/tables/number/:table_number` | Get table by number |
| PUT | `/api/restaurant/tables/:id` | Update table details |
| PUT | `/api/restaurant/tables/:id/status` | Update table status |
| DELETE | `/api/restaurant/tables/:id` | Delete table |

### Modifier Management

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/api/restaurant/modifiers/groups` | Create modifier group |
| GET | `/api/restaurant/modifiers/groups` | List modifier groups |
| GET | `/api/restaurant/modifiers/groups/:id` | Get modifier group with modifiers |
| PUT | `/api/restaurant/modifiers/groups/:id` | Update modifier group |
| DELETE | `/api/restaurant/modifiers/groups/:id` | Delete modifier group |
| POST | `/api/restaurant/modifiers` | Create modifier |
| GET | `/api/restaurant/modifiers/:id` | Get modifier |
| PUT | `/api/restaurant/modifiers/:id` | Update modifier |
| DELETE | `/api/restaurant/modifiers/:id` | Delete modifier |
| GET | `/api/restaurant/modifiers/products/:product_id/groups` | Get modifier groups for product |
| POST | `/api/restaurant/modifiers/products/groups/link` | Link product to modifier group |
| DELETE | `/api/restaurant/modifiers/products/:product_id/groups/:modifier_group_id` | Unlink product from modifier group |

### Kitchen Orders (KOT/KDS)

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/api/restaurant/kitchen/orders` | Create kitchen order |
| GET | `/api/restaurant/kitchen/orders` | List kitchen orders (filterable) |
| GET | `/api/restaurant/kitchen/orders/pending` | Get pending kitchen orders |
| GET | `/api/restaurant/kitchen/orders/active` | Get active kitchen orders (pending + preparing) |
| GET | `/api/restaurant/kitchen/orders/:id` | Get kitchen order by ID |
| PUT | `/api/restaurant/kitchen/orders/:id` | Update kitchen order status |
| PUT | `/api/restaurant/kitchen/orders/items/:id` | Update kitchen order item status |

### WebSocket Endpoint

| Endpoint | Description |
|----------|-------------|
| `/api/ws/kitchen` | WebSocket endpoint for kitchen display real-time updates |

## Real-time Communication

### Kitchen Display WebSocket

The kitchen display system uses WebSocket for real-time updates. When kitchen orders are created or updated, the changes are broadcast to all connected kitchen display terminals.

#### WebSocket Event Format

```json
{
  "event_type": "kitchen_order_created",
  "kitchen_order_id": 123,
  "status": "pending",
  "table_id": 5,
  "table_number": "T5",
  "timestamp": "2024-01-15T10:30:00Z"
}
```

#### Event Types

- `kitchen_order_created`: New kitchen order created
- `kitchen_order_updated`: Kitchen order status updated
- `kitchen_order_item_updated`: Individual item status updated

#### Client Implementation Example

```javascript
const ws = new WebSocket('ws://localhost:8080/api/ws/kitchen');

ws.onopen = () => {
  console.log('Connected to kitchen display');
  // Send ping to keep connection alive
  setInterval(() => {
    ws.send(JSON.stringify({ type: 'ping' }));
  }, 30000);
};

ws.onmessage = (event) => {
  const data = JSON.parse(event.data);
  console.log('Kitchen event:', data);
  
  if (data.event_type === 'kitchen_order_created') {
    // Refresh kitchen display
    fetchKitchenOrders();
  }
};

ws.onerror = (error) => {
  console.error('WebSocket error:', error);
};

ws.onclose = () => {
  console.log('WebSocket connection closed');
  // Implement reconnection logic
};
```

## Integration with Retail Module

### Sales Integration

The restaurant module extends the existing sales table with restaurant-specific fields:

```rust
// When creating a restaurant sale, include table information
POST /api/sales
{
  "customer_id": null,
  "payment_method": "cash",
  "amount_paid": 45.50,
  "items": [...],
  "table_id": 5,              // New field for restaurant tables
  "order_type": "dine_in",   // New field: dine_in, takeaway, delivery
  "guest_count": 4,          // New field
  "special_requests": "No onions" // New field
}
```

### Inventory Integration

Restaurant orders still use the existing inventory system for stock checks and updates. The kitchen order service internally validates product availability and manages stock through the existing inventory service.

### Authentication Integration

Restaurant endpoints use the same JWT authentication and RBAC middleware as the retail module. No separate authentication system is needed.

## Usage Examples

### Creating a Table

```bash
curl -X POST http://localhost:8080/api/restaurant/tables \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -d '{
    "table_number": "T1",
    "capacity": 4,
    "status": "available",
    "zone": "Indoor",
    "location_description": "Near window"
  }'
```

### Creating Modifier Groups

```bash
# Create a modifier group
curl -X POST http://localhost:8080/api/restaurant/modifiers/groups \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -d '{
    "name": "Meat Choice",
    "description": "Choose your meat type",
    "is_required": true,
    "min_selections": 1,
    "max_selections": 1
  }'

# Add modifiers to the group
curl -X POST http://localhost:8080/api/restaurant/modifiers \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -d '{
    "group_id": 1,
    "name": "Chicken",
    "price_adjustment": 0.00
  }'

# Link product to modifier group
curl -X POST http://localhost:8080/api/restaurant/modifiers/products/groups/link \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -d '{
    "product_id": 15,
    "modifier_group_id": 1,
    "is_required": true
  }'
```

### Creating a Kitchen Order

```bash
curl -X POST http://localhost:8080/api/restaurant/kitchen/orders \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -d '{
    "sale_id": 100,
    "table_id": 5,
    "order_type": "dine_in",
    "priority": 5,
    "notes": "Customer is in a hurry",
    "estimated_preparation_time": 15,
    "items": [
      {
        "product_id": 15,
        "quantity": 2,
        "special_instructions": "Extra spicy",
        "selected_modifiers": [
          {
            "modifier_id": 3,
            "quantity": 1
          }
        ]
      }
    ]
  }'
```

### Updating Kitchen Order Status

```bash
# Update order status to "preparing"
curl -X PUT http://localhost:8080/api/restaurant/kitchen/orders/123 \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -d '{
    "status": "preparing"
  }'

# Update individual item status
curl -X PUT http://localhost:8080/api/restaurant/kitchen/orders/items/456 \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -d '{
    "item_status": "ready"
  }'
```

## Testing

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_create_table() {
        // Test table creation logic
    }
    
    #[tokio::test]
    async fn test_kitchen_order_workflow() {
        // Test complete kitchen order workflow
    }
}
```

### Integration Tests

```bash
# Run integration tests
cargo test --test integration_tests

# Test with database
cargo test -- --ignored
```

## Performance Considerations

### Database Indexes

The migration script includes strategic indexes for optimal performance:

- `idx_restaurant_tables_status`: Fast table status queries
- `idx_kitchen_orders_status`: Fast kitchen order status filtering
- `idx_kitchen_orders_priority`: Priority-based ordering for kitchen display
- `idx_kitchen_orders_created_at`: Time-based queries

### Connection Pooling

The restaurant module uses the same database connection pool as the retail module, ensuring efficient resource utilization.

### WebSocket Performance

- Broadcast channel capacity: 100 messages
- Automatic connection cleanup on disconnect
- Ping/pong mechanism for connection health monitoring

## Security Considerations

### Authentication

All restaurant endpoints require JWT authentication (except where explicitly public).

### Authorization

RBAC middleware can be extended to include restaurant-specific permissions:

```rust
// Example: Add restaurant-specific permissions
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Permission {
    // Existing retail permissions
    ProductRead,
    ProductWrite,
    // Restaurant-specific permissions
    TableManage,
    KitchenOrderManage,
    ModifierManage,
}
```

### Input Validation

All restaurant endpoints use the `validator` crate for input validation.

### SQL Injection Prevention

All database queries use parameterized queries via sqlx.

## Troubleshooting

### Common Issues

1. **WebSocket Connection Fails**
   - Ensure the server is running
   - Check CORS configuration
   - Verify the WebSocket endpoint URL

2. **Kitchen Orders Not Broadcasting**
   - Check that the broadcast manager is properly initialized
   - Verify the sender is passed to the kitchen order service
   - Check server logs for broadcast errors

3. **Table Status Not Updating**
   - Verify the table exists
   - Check for concurrent updates
   - Review database constraints

### Debug Mode

Enable debug logging to troubleshoot issues:

```env
RUST_LOG=debug
```

## Migration from Retail to Restaurant

### For Existing Retail Users

1. Run the database migration:
```bash
sqlx migrate run --database-url "postgresql://user:pass@localhost/pos_system"
```

2. The retail module remains completely unchanged
3. Restaurant features are optional and can be used alongside retail features

### Data Migration

If migrating existing retail data to restaurant:

```sql
-- Example: Update existing sales with default order type
UPDATE sales 
SET order_type = 'takeaway' 
WHERE order_type IS NULL;
```

## Future Enhancements

Potential future features for the restaurant module:

- Table reservation system
- Advanced kitchen routing (by station, by chef)
- Menu management with photos
- Customer feedback integration
- Staff scheduling integration
- Advanced reporting (table turnover, kitchen efficiency)
- Multi-location support
- Mobile waiter interface

## Support

For issues and questions:
1. Check this integration guide
2. Review the API documentation
3. Check server logs for error messages
4. Open an issue on the repository

## Summary

The restaurant module provides a complete, production-ready F&B workflow system that:

- ✅ Maintains strict separation from retail logic
- ✅ Uses Domain-Driven Design principles
- ✅ Provides real-time kitchen display updates
- ✅ Integrates seamlessly with existing retail features
- ✅ Follows Rust best practices and idioms
- ✅ Ensures data integrity with ACID transactions
- ✅ Provides comprehensive API endpoints
- ✅ Includes WebSocket support for live updates

The module is ready for production use and can be extended as needed for specific restaurant requirements.
