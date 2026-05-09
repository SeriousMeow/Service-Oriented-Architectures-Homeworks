CREATE TABLE IF NOT EXISTS agg_daily_metrics
(
    metric_date Date,
    metric_name LowCardinality(String),
    metric_value Float64,
    metric_payload String,
    computed_at DateTime64(3, 'UTC')
)
ENGINE = ReplacingMergeTree(computed_at)
ORDER BY (metric_date, metric_name);
