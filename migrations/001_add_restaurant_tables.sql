-- Migration: Add Restaurant & F&B Features to POS System
-- Version: 001
-- Description: Adds table management, product modifiers, and kitchen order tracking

-- Enable UUID extension if not already enabled
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- ============================================
-- 1. ENUM TYPES
-- ============================================

-- Table status enum
CREATE TYPE table_status AS ENUM (
    'available',
    'occupied', 
    'reserved',
    'cleaning',
    'maintenance'
);

-- Kitchen Order Ticket status enum
CREATE TYPE kot_status AS ENUM (
    'pending',
    'preparing',
    'ready',
    'served',
    'cancelled',
    'delayed'
);

-- Order type enum
CREATE TYPE order_type AS ENUM (
    'dine_in',
    'takeaway',
    'delivery'
);

-- ============================================
-- 2. RESTAURANT TABLES
-- ============================================

CREATE TABLE restaurant_tables (
    id SERIAL PRIMARY KEY,
    table_number VARCHAR(20) UNIQUE NOT NULL,
    capacity INT NOT NULL DEFAULT 4 CHECK (capacity > 0),
    status table_status NOT NULL DEFAULT 'available',
    zone VARCHAR(50), -- e.g., 'Indoor', 'Outdoor', 'VIP', 'Bar'
    location_description TEXT,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- Trigger to update updated_at timestamp
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = CURRENT_TIMESTAMP;
    RETURN NEW;
END;
$$ language 'plpgsql';

CREATE TRIGGER update_restaurant_tables_updated_at 
    BEFORE UPDATE ON restaurant_tables 
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- ============================================
-- 3. PRODUCT MODIFIERS & VARIANTS
-- ============================================

-- Modifier groups (categories of modifiers)
CREATE TABLE modifier_groups (
    id SERIAL PRIMARY KEY,
    name VARCHAR(100) NOT NULL,
    description TEXT,
    is_required BOOLEAN NOT NULL DEFAULT FALSE,
    min_selections INT NOT NULL DEFAULT 0 CHECK (min_selections >= 0),
    max_selections INT NOT NULL DEFAULT 1 CHECK (max_selections > 0),
    display_order INT DEFAULT 0,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT valid_selection_range CHECK (min_selections <= max_selections)
);

CREATE TRIGGER update_modifier_groups_updated_at 
    BEFORE UPDATE ON modifier_groups 
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- Individual modifiers (options within groups)
CREATE TABLE modifiers (
    id SERIAL PRIMARY KEY,
    group_id INT NOT NULL REFERENCES modifier_groups(id) ON DELETE CASCADE,
    name VARCHAR(100) NOT NULL,
    description TEXT,
    price_adjustment NUMERIC(12, 2) NOT NULL DEFAULT 0.00 CHECK (price_adjustment >= 0),
    is_available BOOLEAN NOT NULL DEFAULT TRUE,
    display_order INT DEFAULT 0,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

CREATE TRIGGER update_modifiers_updated_at 
    BEFORE UPDATE ON modifiers 
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- Link products to modifier groups
CREATE TABLE product_modifier_groups (
    product_id INT NOT NULL REFERENCES products(id) ON DELETE CASCADE,
    modifier_group_id INT NOT NULL REFERENCES modifier_groups(id) ON DELETE CASCADE,
    is_required BOOLEAN NOT NULL DEFAULT FALSE,
    PRIMARY KEY (product_id, modifier_group_id)
);

-- ============================================
-- 4. EXTEND SALES TABLE FOR RESTAURANT
-- ============================================

-- Add restaurant-specific columns to existing sales table
ALTER TABLE sales 
    ADD COLUMN IF NOT EXISTS table_id INT REFERENCES restaurant_tables(id) ON DELETE SET NULL,
    ADD COLUMN IF NOT EXISTS order_type order_type DEFAULT 'dine_in',
    ADD COLUMN IF NOT EXISTS guest_count INT DEFAULT 1 CHECK (guest_count > 0),
    ADD COLUMN IF NOT EXISTS special_requests TEXT;

-- Add indexes for restaurant queries
CREATE INDEX IF NOT EXISTS idx_sales_table_id ON sales(table_id);
CREATE INDEX IF NOT EXISTS idx_sales_order_type ON sales(order_type);
CREATE INDEX IF NOT EXISTS idx_sales_created_at_restaurant ON sales(created_at) 
    WHERE order_type IN ('dine_in', 'takeaway', 'delivery');

-- ============================================
-- 5. KITCHEN ORDER TICKETS (KOT)
-- ============================================

CREATE TABLE kitchen_orders (
    id SERIAL PRIMARY KEY,
    sale_id INT NOT NULL REFERENCES sales(id) ON DELETE CASCADE,
    table_id INT REFERENCES restaurant_tables(id) ON DELETE SET NULL,
    order_type order_type NOT NULL,
    status kot_status NOT NULL DEFAULT 'pending',
    priority INT DEFAULT 5 CHECK (priority BETWEEN 1 AND 10), -- 1=highest, 10=lowest
    notes TEXT,
    estimated_preparation_time INT, -- in minutes
    actual_preparation_time INT, -- in minutes
    started_at TIMESTAMP WITH TIME ZONE,
    completed_at TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

CREATE TRIGGER update_kitchen_orders_updated_at 
    BEFORE UPDATE ON kitchen_orders 
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- Kitchen order items (item-level tracking)
CREATE TABLE kitchen_order_items (
    id SERIAL PRIMARY KEY,
    kitchen_order_id INT NOT NULL REFERENCES kitchen_orders(id) ON DELETE CASCADE,
    product_id INT REFERENCES products(id) ON DELETE SET NULL,
    product_name VARCHAR(255) NOT NULL, -- Denormalized for historical accuracy
    quantity INT NOT NULL CHECK (quantity > 0),
    unit_price NUMERIC(12, 2) NOT NULL DEFAULT 0.00,
    selected_modifiers JSONB DEFAULT '[]'::jsonb,
    special_instructions TEXT,
    item_status kot_status NOT NULL DEFAULT 'pending',
    sequence_order INT DEFAULT 0, -- For preparation order
    started_at TIMESTAMP WITH TIME ZONE,
    completed_at TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

CREATE TRIGGER update_kitchen_order_items_updated_at 
    BEFORE UPDATE ON kitchen_order_items 
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- ============================================
-- 6. PERFORMANCE INDEXES
-- ============================================

-- Restaurant tables indexes
CREATE INDEX idx_restaurant_tables_status ON restaurant_tables(status);
CREATE INDEX idx_restaurant_tables_zone ON restaurant_tables(zone);
CREATE INDEX idx_restaurant_tables_active ON restaurant_tables(is_active) 
    WHERE is_active = TRUE;

-- Kitchen orders indexes
CREATE INDEX idx_kitchen_orders_status ON kitchen_orders(status);
CREATE INDEX idx_kitchen_orders_created_at ON kitchen_orders(created_at DESC);
CREATE INDEX idx_kitchen_orders_table_id ON kitchen_orders(table_id);
CREATE INDEX idx_kitchen_orders_sale_id ON kitchen_orders(sale_id);
CREATE INDEX idx_kitchen_orders_priority ON kitchen_orders(priority) 
    WHERE status IN ('pending', 'preparing');

-- Kitchen order items indexes
CREATE INDEX idx_kitchen_order_items_status ON kitchen_order_items(item_status);
CREATE INDEX idx_kitchen_order_items_kitchen_order_id ON kitchen_order_items(kitchen_order_id);
CREATE INDEX idx_kitchen_order_items_product_id ON kitchen_order_items(product_id);

-- Modifier indexes
CREATE INDEX idx_modifiers_group_id ON modifiers(group_id);
CREATE INDEX idx_modifiers_active ON modifiers(is_available) 
    WHERE is_available = TRUE;

-- ============================================
-- 7. VIEWS FOR COMMON QUERIES
-- ============================================

-- View for active tables with current orders
CREATE OR REPLACE VIEW active_tables_with_orders AS
SELECT 
    rt.id,
    rt.table_number,
    rt.capacity,
    rt.status,
    rt.zone,
    s.id as current_sale_id,
    s.guest_count,
    s.created_at as order_created_at
FROM restaurant_tables rt
LEFT JOIN sales s ON s.table_id = rt.id AND s.status = 'active'
WHERE rt.is_active = TRUE;

-- View for pending kitchen orders
CREATE OR REPLACE VIEW pending_kitchen_orders AS
SELECT 
    ko.id,
    ko.sale_id,
    ko.table_id,
    ko.order_type,
    ko.status,
    ko.priority,
    ko.notes,
    ko.created_at,
    rt.table_number,
    COUNT(koi.id) as total_items,
    SUM(CASE WHEN koi.item_status = 'pending' THEN 1 ELSE 0 END) as pending_items,
    SUM(CASE WHEN koi.item_status = 'preparing' THEN 1 ELSE 0 END) as preparing_items
FROM kitchen_orders ko
LEFT JOIN restaurant_tables rt ON ko.table_id = rt.id
LEFT JOIN kitchen_order_items koi ON koi.kitchen_order_id = ko.id
WHERE ko.status IN ('pending', 'preparing')
GROUP BY ko.id, ko.sale_id, ko.table_id, ko.order_type, ko.status, 
         ko.priority, ko.notes, ko.created_at, rt.table_number
ORDER BY 
    ko.priority ASC,
    ko.created_at ASC;

-- ============================================
-- 8. SAMPLE DATA (OPTIONAL - FOR DEVELOPMENT)
-- ============================================

-- Insert sample zones (optional)
-- INSERT INTO restaurant_tables (table_number, capacity, status, zone) VALUES
-- ('T1', 4, 'available', 'Indoor'),
-- ('T2', 4, 'available', 'Indoor'),
-- ('T3', 6, 'available', 'Indoor'),
-- ('T4', 2, 'available', 'Outdoor'),
-- ('T5', 8, 'available', 'VIP');

-- ============================================
-- 9. ROW LEVEL SECURITY (OPTIONAL - FOR MULTI-TENANCY)
-- ============================================

-- Uncomment if you need row-level security
-- ALTER TABLE restaurant_tables ENABLE ROW LEVEL SECURITY;
-- ALTER TABLE kitchen_orders ENABLE ROW LEVEL SECURITY;
-- ALTER TABLE kitchen_order_items ENABLE ROW LEVEL SECURITY;

-- ============================================
-- MIGRATION COMPLETE
-- ============================================

-- Verify migration
SELECT 'Restaurant tables migration completed successfully' as status;
