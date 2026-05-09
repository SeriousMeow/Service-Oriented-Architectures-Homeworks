CREATE TABLE IF NOT EXISTS agg_ch_dau_daily
(
    metric_date Date,
    users AggregateFunction(uniq, String)
)
ENGINE = AggregatingMergeTree
PARTITION BY toYYYYMM(metric_date)
ORDER BY metric_date;


CREATE MATERIALIZED VIEW IF NOT EXISTS mv_ch_events_to_dau TO agg_ch_dau_daily
AS SELECT
    toDate(`timestamp`) AS metric_date,
    uniqState(user_id) AS users
FROM movie_events
GROUP BY metric_date;


CREATE TABLE IF NOT EXISTS agg_ch_conversion_daily
(
    metric_date Date,
    started AggregateFunction(sum, UInt64),
    finished AggregateFunction(sum, UInt64)
)
ENGINE = AggregatingMergeTree
PARTITION BY toYYYYMM(metric_date)
ORDER BY metric_date;


CREATE MATERIALIZED VIEW IF NOT EXISTS mv_ch_events_to_conversion TO agg_ch_conversion_daily
AS SELECT
    toDate(`timestamp`) AS metric_date,
    sumState(toUInt64(event_type = 'VIEW_STARTED')) AS started,
    sumState(toUInt64(event_type = 'VIEW_FINISHED')) AS finished
FROM movie_events
GROUP BY metric_date;


CREATE TABLE IF NOT EXISTS agg_ch_avg_watch_daily
(
    metric_date Date,
    avg_watch AggregateFunction(avg, Float64)
)
ENGINE = AggregatingMergeTree
PARTITION BY toYYYYMM(metric_date)
ORDER BY metric_date;


CREATE MATERIALIZED VIEW IF NOT EXISTS mv_ch_events_to_avg_watch TO agg_ch_avg_watch_daily
AS SELECT
    toDate(`timestamp`) AS metric_date,
    avgStateIf(toFloat64(progress_seconds), event_type = 'VIEW_FINISHED') AS avg_watch
FROM movie_events
GROUP BY metric_date;


CREATE TABLE IF NOT EXISTS agg_ch_movie_rank_daily
(
    metric_date Date,
    movie_id String,
    started AggregateFunction(sum, UInt64),
    finished AggregateFunction(sum, UInt64)
)
ENGINE = AggregatingMergeTree
PARTITION BY toYYYYMM(metric_date)
ORDER BY (metric_date, movie_id);


CREATE MATERIALIZED VIEW IF NOT EXISTS mv_ch_events_to_movie_rank TO agg_ch_movie_rank_daily
AS SELECT
    toDate(`timestamp`) AS metric_date,
    movie_id,
    sumState(toUInt64(event_type = 'VIEW_STARTED')) AS started,
    sumState(toUInt64(event_type = 'VIEW_FINISHED')) AS finished
FROM movie_events
GROUP BY metric_date, movie_id;


CREATE TABLE IF NOT EXISTS agg_ch_dau_by_device
(
    metric_date Date,
    device_type LowCardinality(String),
    users AggregateFunction(uniq, String)
)
ENGINE = AggregatingMergeTree
PARTITION BY toYYYYMM(metric_date)
ORDER BY (metric_date, device_type);


CREATE MATERIALIZED VIEW IF NOT EXISTS mv_ch_events_to_dau_device TO agg_ch_dau_by_device
AS SELECT
    toDate(`timestamp`) AS metric_date,
    device_type,
    uniqState(user_id) AS users
FROM movie_events
GROUP BY metric_date, device_type;


CREATE OR REPLACE VIEW v_ch_dau_by_date AS
SELECT
    metric_date,
    toFloat64(uniqMerge(users)) AS dau
FROM agg_ch_dau_daily
GROUP BY metric_date;


CREATE OR REPLACE VIEW v_ch_conversion_by_date AS
SELECT
    metric_date,
    sumMerge(started) AS view_started,
    sumMerge(finished) AS view_finished,
    if(sumMerge(started) = 0, 0.0, toFloat64(sumMerge(finished)) / toFloat64(sumMerge(started))) AS conversion_rate
FROM agg_ch_conversion_daily
GROUP BY metric_date;


CREATE OR REPLACE VIEW v_ch_avg_watch_by_date AS
SELECT
    metric_date,
    avgMerge(avg_watch) AS avg_watch_seconds
FROM agg_ch_avg_watch_daily
GROUP BY metric_date;


CREATE OR REPLACE VIEW v_ch_dau_by_device AS
SELECT
    metric_date,
    device_type,
    toFloat64(uniqMerge(users)) AS dau
FROM agg_ch_dau_by_device
GROUP BY metric_date, device_type;


CREATE OR REPLACE VIEW v_retention_cohort_heatmap AS
WITH
    fv AS (
        SELECT
            user_id,
            min(toDate(`timestamp`)) AS cohort_date
        FROM movie_events
        WHERE event_type = 'VIEW_STARTED'
        GROUP BY user_id
    ),
    sizes AS (
        SELECT
            cohort_date,
            toUInt64(uniq(user_id)) AS cohort_size
        FROM fv
        GROUP BY cohort_date
    ),
    activity AS (
        SELECT DISTINCT
            user_id,
            toDate(`timestamp`) AS d
        FROM movie_events
    ),
    joined AS (
        SELECT
            fv.cohort_date AS cohort_date,
            toUInt8(dateDiff('day', fv.cohort_date, a.d)) AS day_offset,
            fv.user_id AS user_id
        FROM fv
        INNER JOIN activity AS a ON fv.user_id = a.user_id
        WHERE (dateDiff('day', fv.cohort_date, a.d) >= 0) AND (dateDiff('day', fv.cohort_date, a.d) <= 7)
    )
SELECT
    j.cohort_date AS cohort_date,
    j.day_offset AS day_offset,
    max(s.cohort_size) AS cohort_size,
    toUInt64(uniq(j.user_id)) AS active_users,
    if(max(s.cohort_size) = 0, 0.0, toFloat64(uniq(j.user_id)) / toFloat64(max(s.cohort_size))) AS retention
FROM joined AS j
INNER JOIN sizes AS s ON j.cohort_date = s.cohort_date
GROUP BY
    j.cohort_date,
    j.day_offset;


CREATE TABLE IF NOT EXISTS agg_retention_cohort_heatmap
(
    cohort_date Date,
    day_offset UInt8,
    cohort_size UInt64,
    active_users UInt64,
    retention Float64
)
ENGINE = MergeTree
ORDER BY (cohort_date, day_offset);


INSERT INTO agg_retention_cohort_heatmap
SELECT *
FROM v_retention_cohort_heatmap
WHERE 1 = 0;
