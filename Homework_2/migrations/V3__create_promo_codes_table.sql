CREATE TYPE discount_type AS ENUM ('PERCENTAGE', 'FIXED_AMOUNT');

CREATE TABLE promo_codes (
    id               BIGSERIAL PRIMARY KEY,
    code             VARCHAR(20)    NOT NULL UNIQUE,
    discount_type    discount_type  NOT NULL,
    discount_value   DECIMAL(12, 2) NOT NULL CHECK (discount_value >= 0),
    min_order_amount DECIMAL(12, 2) NOT NULL DEFAULT 0 CHECK (min_order_amount >= 0),
    max_uses         INTEGER        NOT NULL CHECK (max_uses > 0),
    current_uses     INTEGER        NOT NULL DEFAULT 0 CHECK (current_uses >= 0),
    valid_from       TIMESTAMPTZ    NOT NULL,
    valid_until      TIMESTAMPTZ    NOT NULL,
    active           BOOLEAN        NOT NULL DEFAULT TRUE,
    CONSTRAINT check_valid_dates CHECK (valid_until > valid_from),
    CONSTRAINT check_uses CHECK (current_uses <= max_uses)
);

CREATE INDEX idx_promo_codes_code ON promo_codes (code);
CREATE INDEX idx_promo_codes_active ON promo_codes (active);

ALTER TABLE orders
    ADD CONSTRAINT fk_orders_promo_code
    FOREIGN KEY (promo_code_id) REFERENCES promo_codes (id);
