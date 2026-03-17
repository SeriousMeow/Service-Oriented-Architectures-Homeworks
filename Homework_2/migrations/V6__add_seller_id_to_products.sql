ALTER TABLE products
    ADD COLUMN seller_id BIGINT REFERENCES users (id);

CREATE INDEX idx_products_seller_id ON products (seller_id);
