-- Default new sales to 'retail' now that the enum value exists (added in 006).
-- Plain POS sales omit order_type; dine-in flows set it explicitly.
ALTER TABLE sales ALTER COLUMN order_type SET DEFAULT 'retail';
