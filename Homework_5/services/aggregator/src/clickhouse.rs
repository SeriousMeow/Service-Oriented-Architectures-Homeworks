use anyhow::Context;
use reqwest::Client;
use serde::Deserialize;

#[derive(Clone)]
pub struct ClickhouseHttp {
    client: Client,
    base: String,
    user: String,
    password: String,
}

impl ClickhouseHttp {
    pub fn new(base: String, user: String, password: String) -> anyhow::Result<Self> {
        let client = Client::builder()
            .build()
            .context("build reqwest client")?;
        Ok(Self {
            client,
            base: base.trim_end_matches('/').to_string(),
            user,
            password,
        })
    }

    async fn post_query(&self, sql: &str) -> anyhow::Result<String> {
        let url = format!("{}/?default_format=JSONEachRow", self.base);
        let mut req = self.client.post(url).body(sql.to_string());
        if !self.password.is_empty() {
            req = req.basic_auth(&self.user, Some(&self.password));
        }
        let resp = req.send().await.context("clickhouse http")?;
        let status = resp.status();
        let body = resp.text().await.context("clickhouse body")?;
        if !status.is_success() {
            anyhow::bail!("clickhouse error {status}: {body}");
        }
        Ok(body)
    }

    pub async fn count_events_for_day(&self, date: &str) -> anyhow::Result<u64> {
        let sql = format!(
            "SELECT count() AS v FROM movie_events WHERE toDate(timestamp) = toDate('{date}') FORMAT JSONEachRow"
        );
        let body = self.post_query(&sql).await?;
        let line = body.lines().find(|l| !l.trim().is_empty()).unwrap_or("");
        if line.is_empty() {
            return Ok(0);
        }
        let row: CountRow = serde_json::from_str(line).context("parse count row")?;
        let v = row
            .v
            .as_ref()
            .and_then(|x| {
                x.as_u64()
                    .or_else(|| x.as_i64().and_then(|i| u64::try_from(i).ok()))
                    .or_else(|| x.as_str().and_then(|s| s.parse::<u64>().ok()))
            })
            .unwrap_or(0);
        Ok(v)
    }

    pub async fn metric_dau(&self, date: &str) -> anyhow::Result<f64> {
        let sql = format!(
            "SELECT dau AS v FROM v_ch_dau_by_date WHERE metric_date = toDate('{date}') FORMAT JSONEachRow"
        );
        self.one_f64(&sql).await
    }

    pub async fn metric_avg_watch(&self, date: &str) -> anyhow::Result<f64> {
        let sql = format!(
            r#"SELECT if(isNaN(avg_watch_seconds), 0.0, toFloat64(avg_watch_seconds)) AS v
FROM v_ch_avg_watch_by_date
WHERE metric_date = toDate('{date}')
FORMAT JSONEachRow"#
        );
        self.one_f64(&sql).await
    }

    pub async fn metric_conversion(&self, date: &str) -> anyhow::Result<f64> {
        let sql = format!(
            r#"SELECT conversion_rate AS v
FROM v_ch_conversion_by_date
WHERE metric_date = toDate('{date}')
FORMAT JSONEachRow"#
        );
        self.one_f64(&sql).await
    }

    pub async fn metric_retention(&self, date: &str) -> anyhow::Result<(f64, f64)> {
        let sql = format!(
            r#"SELECT
  coalesce(maxIf(retention, day_offset = 1), 0.0) AS d1,
  coalesce(maxIf(retention, day_offset = 7), 0.0) AS d7
FROM v_retention_cohort_heatmap
WHERE cohort_date = toDate('{date}')
FORMAT JSONEachRow"#
        );
        let body = self.post_query(&sql).await?;
        let line = body.lines().find(|l| !l.trim().is_empty()).unwrap_or("");
        if line.is_empty() {
            return Ok((0.0, 0.0));
        }
        let row: RetentionRow = serde_json::from_str(line).context("parse retention")?;
        Ok((row.d1, row.d7))
    }

    pub async fn top_movies(&self, date: &str, limit: u8) -> anyhow::Result<Vec<(String, u64)>> {
        let sql = format!(
            r#"SELECT
  movie_id,
  toUInt64(sumMerge(finished)) AS c
FROM agg_ch_movie_rank_daily
WHERE metric_date = toDate('{date}')
GROUP BY movie_id
ORDER BY c DESC
LIMIT {limit}
FORMAT JSONEachRow"#
        );
        let body = self.post_query(&sql).await?;
        let mut out = Vec::new();
        for line in body.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let row: TopMovieRow = serde_json::from_str(line).context("parse top movie row")?;
            out.push((row.movie_id, row.c));
        }
        Ok(out)
    }

    pub async fn insert_agg_rows(&self, rows: &[AggRow]) -> anyhow::Result<()> {
        if rows.is_empty() {
            return Ok(());
        }
        let mut payload = String::from(
            "INSERT INTO agg_daily_metrics (metric_date, metric_name, metric_value, metric_payload, computed_at) FORMAT JSONEachRow\n",
        );
        for r in rows {
            let line = serde_json::to_string(r).context("serialize agg row")?;
            payload.push_str(&line);
            payload.push('\n');
        }
        self.post_query(&payload).await?;
        Ok(())
    }

    async fn one_f64(&self, sql: &str) -> anyhow::Result<f64> {
        let body = self.post_query(sql).await?;
        let line = body.lines().find(|l| !l.trim().is_empty()).unwrap_or("");
        if line.is_empty() {
            return Ok(0.0);
        }
        let row: FloatRow = serde_json::from_str(line).context("parse float row")?;
        let v = row
            .v
            .as_ref()
            .and_then(|x| x.as_f64().or_else(|| x.as_i64().map(|i| i as f64)))
            .unwrap_or(0.0);
        Ok(v)
    }
}

#[derive(Deserialize)]
struct CountRow {
    v: Option<serde_json::Value>,
}

#[derive(Deserialize)]
struct FloatRow {
    v: Option<serde_json::Value>,
}

#[derive(Deserialize)]
struct RetentionRow {
    d1: f64,
    d7: f64,
}

#[derive(Deserialize)]
struct TopMovieRow {
    movie_id: String,
    c: u64,
}

#[derive(serde::Serialize)]
pub struct AggRow {
    pub metric_date: String,
    pub metric_name: String,
    pub metric_value: f64,
    pub metric_payload: String,
    pub computed_at: String,
}
