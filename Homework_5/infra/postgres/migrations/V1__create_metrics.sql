CREATE TABLE IF NOT EXISTS metrics (
    metric_date DATE NOT NULL,
    metric_name TEXT NOT NULL,
    metric_value DOUBLE PRECISION,
    metric_payload JSONB,
    computed_at TIMESTAMPTZ NOT NULL,
    CONSTRAINT metrics_pkey PRIMARY KEY (metric_date, metric_name)
);

CREATE INDEX IF NOT EXISTS metrics_computed_at_idx ON metrics (computed_at DESC);
