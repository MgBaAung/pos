-- Add a 'retail' order type so plain POS sales are not mislabeled as dine_in.
-- ALTER TYPE ... ADD VALUE cannot be used in the same transaction that consumes
-- the new value, so the sales.order_type default is changed in a later migration.
ALTER TYPE order_type ADD VALUE IF NOT EXISTS 'retail';
