use std::net::SocketAddr;
use std::time::Duration;

#[derive(Clone)]
pub struct Config {
    pub bind_addr: SocketAddr,
    pub export_interval: Duration,
    pub s3_endpoint: String,
    pub s3_region: String,
    pub s3_bucket: String,
    pub aws_access_key: String,
    pub aws_secret_key: String,
}

impl Config {
    pub fn from_env() -> Result<Self, String> {
        let port: u16 = std::env::var("EXPORTER_PORT")
            .unwrap_or_else(|_| "8082".to_string())
            .parse()
            .map_err(|_| "EXPORTER_PORT must be a u16")?;
        let bind_addr: SocketAddr = ([0, 0, 0, 0], port).into();

        let export_sec: u64 = std::env::var("EXPORT_INTERVAL_SEC")
            .unwrap_or_else(|_| "3600".to_string())
            .parse()
            .map_err(|_| "EXPORT_INTERVAL_SEC must be a positive integer")?;
        if export_sec == 0 {
            return Err("EXPORT_INTERVAL_SEC must be > 0".into());
        }

        let s3_endpoint = std::env::var("S3_ENDPOINT")
            .map_err(|_| "S3_ENDPOINT is required (e.g. http://minio:9000)")?;
        let s3_region = std::env::var("S3_REGION").unwrap_or_else(|_| "us-east-1".to_string());
        let s3_bucket =
            std::env::var("S3_BUCKET").unwrap_or_else(|_| "movie-analytics".to_string());
        let aws_access_key = std::env::var("AWS_ACCESS_KEY_ID")
            .map_err(|_| "AWS_ACCESS_KEY_ID is required for S3/MinIO")?;
        let aws_secret_key = std::env::var("AWS_SECRET_ACCESS_KEY")
            .map_err(|_| "AWS_SECRET_ACCESS_KEY is required for S3/MinIO")?;

        Ok(Self {
            bind_addr,
            export_interval: Duration::from_secs(export_sec),
            s3_endpoint,
            s3_region,
            s3_bucket,
            aws_access_key,
            aws_secret_key,
        })
    }
}
