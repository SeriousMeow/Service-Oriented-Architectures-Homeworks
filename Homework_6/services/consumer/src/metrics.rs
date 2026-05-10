use prometheus::{
    Encoder, Histogram, HistogramOpts, IntCounter, IntCounterVec, IntGaugeVec, Registry,
    TextEncoder,
};

#[derive(Clone)]
pub struct Metrics {
    registry: Registry,
    pub consumer_lag: IntGaugeVec,
    pub events_processed_total: IntCounterVec,
    pub event_processing_duration_seconds: Histogram,
    pub cassandra_write_errors_total: IntCounter,
}

impl Metrics {
    pub fn new() -> Result<Self, String> {
        let registry = Registry::new();
        let consumer_lag = IntGaugeVec::new(
            prometheus::Opts::new("consumer_lag", "Consumer lag by partition"),
            &["partition"],
        )
        .map_err(|e| e.to_string())?;
        let events_processed_total = IntCounterVec::new(
            prometheus::Opts::new("events_processed_total", "Processed events by event type"),
            &["event_type"],
        )
        .map_err(|e| e.to_string())?;
        let event_processing_duration_seconds = Histogram::with_opts(HistogramOpts::new(
            "event_processing_duration_seconds",
            "Processing duration of events",
        ))
        .map_err(|e| e.to_string())?;
        let cassandra_write_errors_total = IntCounter::new(
            "cassandra_write_errors_total",
            "Cassandra write errors total",
        )
        .map_err(|e| e.to_string())?;

        registry
            .register(Box::new(consumer_lag.clone()))
            .map_err(|e| e.to_string())?;
        registry
            .register(Box::new(events_processed_total.clone()))
            .map_err(|e| e.to_string())?;
        registry
            .register(Box::new(event_processing_duration_seconds.clone()))
            .map_err(|e| e.to_string())?;
        registry
            .register(Box::new(cassandra_write_errors_total.clone()))
            .map_err(|e| e.to_string())?;

        Ok(Self {
            registry,
            consumer_lag,
            events_processed_total,
            event_processing_duration_seconds,
            cassandra_write_errors_total,
        })
    }

    pub fn render(&self) -> Result<String, String> {
        let metric_families = self.registry.gather();
        let encoder = TextEncoder::new();
        let mut buffer = Vec::new();
        encoder
            .encode(&metric_families, &mut buffer)
            .map_err(|e| e.to_string())?;
        String::from_utf8(buffer).map_err(|e| e.to_string())
    }
}
