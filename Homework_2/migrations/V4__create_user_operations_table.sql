CREATE TYPE operation_type AS ENUM ('CREATE_ORDER', 'UPDATE_ORDER');

CREATE TABLE user_operations (
    id             BIGSERIAL PRIMARY KEY,
    user_id        BIGINT         NOT NULL,
    operation_type operation_type NOT NULL,
    created_at     TIMESTAMPTZ    NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_user_operations_user_id ON user_operations (user_id);
CREATE INDEX idx_user_operations_user_type ON user_operations (user_id, operation_type);
CREATE INDEX idx_user_operations_created_at ON user_operations (created_at);
