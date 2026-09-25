-- Migration: Dine-in checkout support on POS sales
-- Version: 004
-- Description: Lets a POS sale line carry the chosen product modifiers so the
--              dine-in/takeaway flow can record and price them. The sales-level
--              columns (table_id, order_type, guest_count, special_requests) are
--              already added by migration 001; this only extends sale_items.

ALTER TABLE sale_items
    ADD COLUMN IF NOT EXISTS selected_modifiers JSONB NOT NULL DEFAULT '[]'::jsonb;

CREATE INDEX IF NOT EXISTS idx_sale_items_selected_modifiers
    ON sale_items USING GIN (selected_modifiers);

SELECT 'Dine-in sale_items modifier migration completed successfully' AS status;
