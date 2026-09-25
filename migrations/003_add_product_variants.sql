-- Migration: Add Product Variants (Size / Color)
-- Version: 003
-- Description: Adds per-variant SKU, pricing and stock for a base product
--              (e.g. a T-Shirt with Size x Color combinations).

-- ============================================
-- 1. PRODUCT VARIANTS TABLE
-- ============================================

CREATE TABLE product_variants (
    id SERIAL PRIMARY KEY,
    product_id INT NOT NULL REFERENCES products(id) ON DELETE CASCADE,
    sku VARCHAR(100) UNIQUE NOT NULL,
    barcode VARCHAR(100),
    name VARCHAR(150) NOT NULL,
    size VARCHAR(50) NOT NULL DEFAULT '',
    color VARCHAR(50) NOT NULL DEFAULT '',
    cost_price NUMERIC(12, 2) NOT NULL DEFAULT 0.00 CHECK (cost_price >= 0),
    selling_price NUMERIC(12, 2) NOT NULL CHECK (selling_price >= 0),
    stock_quantity INT NOT NULL DEFAULT 0 CHECK (stock_quantity >= 0),
    low_stock_threshold INT NOT NULL DEFAULT 0 CHECK (low_stock_threshold >= 0),
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    -- A product cannot have two variants with the same size/color combination.
    CONSTRAINT uq_product_variant_option UNIQUE (product_id, size, color)
);

-- Reuse the update_updated_at_column() function created in migration 001.
CREATE TRIGGER update_product_variants_updated_at
    BEFORE UPDATE ON product_variants
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- ============================================
-- 2. INDEXES
-- ============================================

CREATE INDEX idx_product_variants_product_id ON product_variants(product_id);
CREATE INDEX idx_product_variants_sku ON product_variants(sku);
CREATE INDEX idx_product_variants_barcode ON product_variants(barcode)
    WHERE barcode IS NOT NULL;
CREATE INDEX idx_product_variants_active ON product_variants(is_active)
    WHERE is_active = TRUE;

-- ============================================
-- MIGRATION COMPLETE
-- ============================================

SELECT 'Product variants migration completed successfully' as status;
