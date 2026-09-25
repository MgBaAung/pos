-- Migration: Loyalty redemption on sales
-- Version: 005
-- Description: Records loyalty points redeemed and the resulting discount on a
--              sale so checkout redemption is auditable and printable.

ALTER TABLE sales
    ADD COLUMN IF NOT EXISTS loyalty_discount NUMERIC(12, 2) NOT NULL DEFAULT 0.00,
    ADD COLUMN IF NOT EXISTS points_redeemed INT NOT NULL DEFAULT 0;

SELECT 'Sales loyalty redemption migration completed successfully' AS status;
