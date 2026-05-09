use anyhow::{bail, Context, Result};

use crate::test::common::Config;

pub async fn verify_subject(
    client: &reqwest::Client,
    cfg: &Config,
    schema_subject: &str,
) -> Result<()> {
    let schema_url = format!(
        "{}/subjects/{}/versions/latest",
        cfg.schema_registry_url.trim_end_matches('/'),
        schema_subject
    );
    let schema_resp = client
        .get(&schema_url)
        .send()
        .await
        .with_context(|| format!("GET {}", schema_url))?;
    if !schema_resp.status().is_success() {
        bail!(
            "schema-registry subject check failed: {}",
            schema_resp.text().await.unwrap_or_default()
        );
    }
    Ok(())
}
