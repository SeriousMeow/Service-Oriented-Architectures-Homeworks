CREATE TYPE order_status AS ENUM ('CREATED', 'PAYMENT_PENDING', 'PAID', 'SHIPPED', 'COMPLETED', 'CANCELED');

CREATE TABLE orders (
    id               BIGSERIAL PRIMARY KEY,
    user_id          BIGINT         NOT NULL,
    status           order_status   NOT NULL DEFAULT 'CREATED',
    promo_code_id    BIGINT,
    total_amount     DECIMAL(12, 2) NOT NULL CHECK (total_amount >= 0),
    discount_amount  DECIMAL(12, 2) NOT NULL DEFAULT 0 CHECK (discount_amount >= 0),
    created_at       TIMESTAMPTZ    NOT NULL DEFAULT NOW(),
    updated_at       TIMESTAMPTZ    NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_orders_user_id ON orders (user_id);
CREATE INDEX idx_orders_status ON orders (status);
CREATE INDEX idx_orders_user_status ON orders (user_id, status);

CREATE TABLE order_items (
    id             BIGSERIAL PRIMARY KEY,
    order_id       BIGINT         NOT NULL REFERENCES orders (id) ON DELETE CASCADE,
    product_id     BIGINT         NOT NULL REFERENCES products (id),
    quantity       INTEGER        NOT NULL CHECK (quantity > 0),
    price_at_order DECIMAL(12, 2) NOT NULL CHECK (price_at_order >= 0)
);

CREATE INDEX idx_order_items_order_id ON order_items (order_id);
CREATE INDEX idx_order_items_product_id ON order_items (product_id);

CREATE TRIGGER orders_set_updated_at
    BEFORE UPDATE ON orders
    FOR EACH ROW
    EXECUTE FUNCTION set_updated_at();
