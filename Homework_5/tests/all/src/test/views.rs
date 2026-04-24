use anyhow::Result;

use crate::test::common::{run_clickhouse_query, Config};

pub async fn verify_clickhouse_views(client: &reqwest::Client, cfg: &Config) -> Result<()> {
    run_clickhouse_query(
        client,
        cfg,
        "SELECT metric_date, dau FROM v_ch_dau_by_date ORDER BY metric_date DESC LIMIT 3 FORMAT JSONEachRow",
    )
    .await?;
    run_clickhouse_query(
        client,
        cfg,
        "SELECT metric_date, conversion_rate FROM v_ch_conversion_by_date ORDER BY metric_date DESC LIMIT 3 FORMAT JSONEachRow",
    )
    .await?;
    Ok(())
}
