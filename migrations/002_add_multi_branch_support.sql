-- Migration: Add Multi-Branch Support
-- Version: 002
-- Description: Adds branch management and multi-tenant support to POS system

-- ============================================
-- 1. BRANCHES TABLE
-- ============================================

CREATE TABLE branches (
    id SERIAL PRIMARY KEY,
    name VARCHAR(100) NOT NULL,
    code VARCHAR(20) UNIQUE NOT NULL,
    address TEXT,
    phone VARCHAR(20),
    email VARCHAR(100),
    manager_name VARCHAR(100),
    manager_phone VARCHAR(20),
    tax_id VARCHAR(50),
    business_license VARCHAR(100),
    opening_time TIME,
    closing_time TIME,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    is_main_branch BOOLEAN NOT NULL DEFAULT FALSE,
    settings JSONB DEFAULT '{}'::jsonb,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT valid_time_range CHECK (opening_time < closing_time OR opening_time IS NULL OR closing_time IS NULL)
);

CREATE TRIGGER update_branches_updated_at 
    BEFORE UPDATE ON branches 
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- Indexes for branches
CREATE INDEX idx_branches_code ON branches(code);
CREATE INDEX idx_branches_active ON branches(is_active) WHERE is_active = TRUE;
CREATE INDEX idx_branches_main ON branches(is_main_branch) WHERE is_main_branch = TRUE;

-- ============================================
-- 2. UPDATE USERS TABLE WITH BRANCH
-- ============================================

ALTER TABLE users 
    ADD COLUMN IF NOT EXISTS branch_id INT REFERENCES branches(id) ON DELETE SET NULL,
    ADD COLUMN IF NOT EXISTS can_access_all_branches BOOLEAN NOT NULL DEFAULT FALSE;

CREATE INDEX idx_users_branch_id ON users(branch_id);
CREATE INDEX idx_users_all_branches ON users(can_access_all_branches) WHERE can_access_all_branches = TRUE;

-- ============================================
-- 3. UPDATE PRODUCTS TABLE WITH BRANCH
-- ============================================

ALTER TABLE products 
    ADD COLUMN IF NOT EXISTS branch_id INT REFERENCES branches(id) ON DELETE CASCADE,
    ADD COLUMN IF NOT EXISTS is_global_product BOOLEAN NOT NULL DEFAULT FALSE;

CREATE INDEX idx_products_branch_id ON products(branch_id);
CREATE INDEX idx_products_global ON products(is_global_product) WHERE is_global_product = TRUE;

-- ============================================
-- 4. UPDATE CATEGORIES TABLE WITH BRANCH
-- ============================================

ALTER TABLE categories 
    ADD COLUMN IF NOT EXISTS branch_id INT REFERENCES branches(id) ON DELETE CASCADE,
    ADD COLUMN IF NOT EXISTS is_global_category BOOLEAN NOT NULL DEFAULT FALSE;

CREATE INDEX idx_categories_branch_id ON categories(branch_id);
CREATE INDEX idx_categories_global ON categories(is_global_category) WHERE is_global_category = TRUE;

-- ============================================
-- 5. UPDATE CUSTOMERS TABLE WITH BRANCH
-- ============================================

ALTER TABLE customers 
    ADD COLUMN IF NOT EXISTS branch_id INT REFERENCES branches(id) ON DELETE SET NULL,
    ADD COLUMN IF NOT EXISTS is_global_customer BOOLEAN NOT NULL DEFAULT FALSE;

CREATE INDEX idx_customers_branch_id ON customers(branch_id);
CREATE INDEX idx_customers_global ON customers(is_global_customer) WHERE is_global_customer = TRUE;

-- ============================================
-- 6. UPDATE SALES TABLE WITH BRANCH
-- ============================================

ALTER TABLE sales 
    ADD COLUMN IF NOT EXISTS branch_id INT REFERENCES branches(id) ON DELETE RESTRICT;

CREATE INDEX idx_sales_branch_id ON sales(branch_id);
CREATE INDEX idx_sales_branch_date ON sales(branch_id, created_at);

-- ============================================
-- 7. UPDATE INVENTORY LOGS TABLE WITH BRANCH
-- ============================================

ALTER TABLE inventory_logs 
    ADD COLUMN IF NOT EXISTS branch_id INT REFERENCES branches(id) ON DELETE RESTRICT;

CREATE INDEX idx_inventory_logs_branch_id ON inventory_logs(branch_id);

-- ============================================
-- 8. UPDATE RESTAURANT TABLES WITH BRANCH
-- ============================================

ALTER TABLE restaurant_tables 
    ADD COLUMN IF NOT EXISTS branch_id INT REFERENCES branches(id) ON DELETE CASCADE;

CREATE INDEX idx_restaurant_tables_branch_id ON restaurant_tables(branch_id);

-- ============================================
-- 9. UPDATE MODIFIER TABLES WITH BRANCH
-- ============================================

ALTER TABLE modifier_groups 
    ADD COLUMN IF NOT EXISTS branch_id INT REFERENCES branches(id) ON DELETE CASCADE,
    ADD COLUMN IF NOT EXISTS is_global_modifier BOOLEAN NOT NULL DEFAULT FALSE;

CREATE INDEX idx_modifier_groups_branch_id ON modifier_groups(branch_id);
CREATE INDEX idx_modifier_groups_global ON modifier_groups(is_global_modifier) WHERE is_global_modifier = TRUE;

-- ============================================
-- 10. ADD USER-BRANCH ASSIGNMENT TABLE
-- ============================================

CREATE TABLE user_branch_assignments (
    id SERIAL PRIMARY KEY,
    user_id INT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    branch_id INT NOT NULL REFERENCES branches(id) ON DELETE CASCADE,
    assigned_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    assigned_by INT REFERENCES users(id) ON DELETE SET NULL,
    is_primary BOOLEAN NOT NULL DEFAULT FALSE,
    UNIQUE(user_id, branch_id)
);

CREATE INDEX idx_user_branch_assignments_user_id ON user_branch_assignments(user_id);
CREATE INDEX idx_user_branch_assignments_branch_id ON user_branch_assignments(branch_id);
CREATE INDEX idx_user_branch_assignments_primary ON user_branch_assignments(user_id, is_primary) WHERE is_primary = TRUE;

-- ============================================
-- 11. ADD BRANCH PERFORMANCE VIEW
-- ============================================

CREATE OR REPLACE VIEW branch_performance AS
SELECT 
    b.id as branch_id,
    b.name as branch_name,
    b.code as branch_code,
    COUNT(DISTINCT s.id) as total_sales,
    COALESCE(SUM(s.grand_total), 0) as total_revenue,
    COALESCE(AVG(s.grand_total), 0) as average_order_value,
    COUNT(DISTINCT s.cashier_id) as active_cashiers,
    COUNT(DISTINCT c.id) as total_customers,
    COUNT(DISTINCT p.id) as total_products,
    b.is_active,
    b.created_at
FROM branches b
LEFT JOIN sales s ON s.branch_id = b.id AND s.status = 'completed'
LEFT JOIN customers c ON c.branch_id = b.id
LEFT JOIN products p ON p.branch_id = b.id
GROUP BY b.id, b.name, b.code, b.is_active, b.created_at
ORDER BY total_revenue DESC;

-- ============================================
-- 12. ADD BRANCH COMPARISON VIEW
-- ============================================

CREATE OR REPLACE VIEW branch_comparison AS
SELECT 
    b1.id as branch_id,
    b1.name as branch_name,
    b1.total_revenue as current_revenue,
    b2.total_revenue as previous_revenue,
    CASE 
        WHEN b2.total_revenue = 0 THEN 0
        ELSE ROUND(((b1.total_revenue - b2.total_revenue) / b2.total_revenue * 100), 2)
    END as revenue_growth_percentage,
    b1.total_sales as current_sales,
    b2.total_sales as previous_sales,
    CASE 
        WHEN b2.total_sales = 0 THEN 0
        ELSE ROUND(((b1.total_sales - b2.total_sales)::numeric / b2.total_sales * 100), 2)
    END as sales_growth_percentage
FROM (
    SELECT 
        b.id, b.name,
        COUNT(DISTINCT s.id) as total_sales,
        COALESCE(SUM(s.grand_total), 0) as total_revenue
    FROM branches b
    LEFT JOIN sales s ON s.branch_id = b.id 
        AND s.status = 'completed'
        AND s.created_at >= CURRENT_DATE - INTERVAL '30 days'
    GROUP BY b.id, b.name
) b1
LEFT JOIN (
    SELECT 
        b.id,
        COUNT(DISTINCT s.id) as total_sales,
        COALESCE(SUM(s.grand_total), 0) as total_revenue
    FROM branches b
    LEFT JOIN sales s ON s.branch_id = b.id 
        AND s.status = 'completed'
        AND s.created_at >= CURRENT_DATE - INTERVAL '60 days'
        AND s.created_at < CURRENT_DATE - INTERVAL '30 days'
    GROUP BY b.id
) b2 ON b1.id = b2.id;

-- ============================================
-- 13. ADD USER BRANCH ACCESS VIEW
-- ============================================

CREATE OR REPLACE VIEW user_branch_access AS
SELECT 
    u.id as user_id,
    u.username,
    u.full_name,
    u.role,
    u.can_access_all_branches,
    COALESCE(array_agg(b.id ORDER BY b.name), ARRAY[]::INT[]) as accessible_branch_ids,
    COALESCE(array_agg(b.name ORDER BY b.name), ARRAY[]::TEXT[]) as accessible_branch_names,
    COALESCE(
        array_agg(b.name ORDER BY b.name) FILTER (WHERE uba.is_primary = TRUE), 
        ARRAY[]::TEXT[]
    ) as primary_branch_name
FROM users u
LEFT JOIN user_branch_assignments uba ON uba.user_id = u.id
LEFT JOIN branches b ON b.id = uba.branch_id AND b.is_active = TRUE
WHERE u.is_active = TRUE
GROUP BY u.id, u.username, u.full_name, u.role, u.can_access_all_branches;

-- ============================================
-- 14. ADD BRANCH INVENTORY SUMMARY VIEW
-- ============================================

CREATE OR REPLACE VIEW branch_inventory_summary AS
SELECT 
    b.id as branch_id,
    b.name as branch_name,
    COUNT(DISTINCT p.id) as total_products,
    COUNT(DISTINCT CASE WHEN p.stock_quantity <= p.low_stock_threshold THEN p.id END) as low_stock_products,
    COUNT(DISTINCT CASE WHEN p.stock_quantity = 0 THEN p.id END) as out_of_stock_products,
    COALESCE(SUM(p.stock_quantity * p.cost_price), 0) as total_inventory_value,
    COALESCE(SUM(p.stock_quantity * p.cost_price), 0) as total_cost_value
FROM branches b
LEFT JOIN products p ON p.branch_id = b.id
WHERE b.is_active = TRUE
GROUP BY b.id, b.name
ORDER BY total_inventory_value DESC;

-- ============================================
-- 15. SAMPLE DATA FOR TESTING
-- ============================================

-- Insert main branch
INSERT INTO branches (name, code, address, phone, email, is_main_branch, is_active) VALUES
('Main Branch', 'MAIN', '123 Main Street, Yangon', '+951234567890', 'main@pos-system.com', TRUE, TRUE);

-- Insert additional branches
INSERT INTO branches (name, code, address, phone, email, manager_name, is_active) VALUES
('Downtown Branch', 'DT', '45 Downtown Street, Yangon', '+951234567891', 'downtown@pos-system.com', 'John Doe', TRUE),
('Mandalay Branch', 'MDL', '78 Mandalay Road, Mandalay', '+951234567892', 'mandalay@pos-system.com', 'Jane Smith', TRUE);

-- ============================================
-- 16. ROW LEVEL SECURITY SETUP
-- ============================================

-- Enable RLS on sensitive tables
ALTER TABLE branches ENABLE ROW LEVEL SECURITY;
ALTER TABLE user_branch_assignments ENABLE ROW LEVEL SECURITY;

-- Create policies for branch access
CREATE POLICY branch_select_policy ON branches
    FOR SELECT
    USING (
        is_active = TRUE OR 
        -- Admins can see all branches
        EXISTS (
            SELECT 1 FROM users 
            WHERE id = current_setting('app.current_user_id', TRUE)::INT 
            AND role = 'admin'
        )
    );

CREATE POLICY branch_modify_policy ON branches
    FOR ALL
    USING (
        EXISTS (
            SELECT 1 FROM users 
            WHERE id = current_setting('app.current_user_id', TRUE)::INT 
            AND role = 'admin'
        )
    );

-- ============================================
-- MIGRATION COMPLETE
-- ============================================

-- Verify migration
SELECT 'Multi-branch support migration completed successfully' as status;
