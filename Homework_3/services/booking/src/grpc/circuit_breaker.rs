use std::collections::VecDeque;
use std::time::{Duration, Instant};

use tokio::sync::Mutex;
use tonic::{Code, Status};

#[derive(Debug, Clone, Copy)]
pub enum WindowMode {
    /// Open after N consecutive failures.
    Consecutive,
    /// Open when failures within last window reach threshold.
    Rolling,
}

impl WindowMode {
    pub fn from_env() -> Self {
        match std::env::var("CIRCUIT_BREAKER_WINDOW_MODE")
            .unwrap_or_else(|_| "rolling".to_string())
            .to_lowercase()
            .as_str()
        {
            "consecutive" => Self::Consecutive,
            _ => Self::Rolling,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    pub window_mode: WindowMode,
    pub window_size: usize,
    pub error_threshold: usize,
    pub open_timeout: Duration,
}

impl CircuitBreakerConfig {
    pub fn from_env() -> Self {
        let window_mode = WindowMode::from_env();
        let window_size = std::env::var("CIRCUIT_BREAKER_WINDOW_SIZE")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(10)
            .max(1);
        let error_threshold = std::env::var("CIRCUIT_BREAKER_ERROR_THRESHOLD")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(5)
            .max(1);
        let open_timeout = Duration::from_millis(
            std::env::var("CIRCUIT_BREAKER_OPEN_TIMEOUT_MS")
                .ok()
                .and_then(|v| v.parse::<u64>().ok())
                .unwrap_or(5_000),
        );

        Self {
            window_mode,
            window_size,
            error_threshold,
            open_timeout,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    Closed,
    Open,
    HalfOpen,
}

#[derive(Debug)]
struct Inner {
    state: State,
    opened_at: Option<Instant>,
    half_open_probe_in_flight: bool,

    // Metrics for CLOSED.
    window: VecDeque<bool>, // true = failure, false = success
    consecutive_failures: usize,
}

#[derive(Debug, Clone)]
pub struct CircuitBreaker {
    cfg: CircuitBreakerConfig,
    inner: std::sync::Arc<Mutex<Inner>>,
}

impl CircuitBreaker {
    pub fn new(cfg: CircuitBreakerConfig) -> Self {
        Self {
            cfg,
            inner: std::sync::Arc::new(Mutex::new(Inner {
                state: State::Closed,
                opened_at: None,
                half_open_probe_in_flight: false,
                window: VecDeque::new(),
                consecutive_failures: 0,
            })),
        }
    }

    fn should_count_failure(code: Code) -> bool {
        matches!(
            code,
            Code::Unavailable | Code::DeadlineExceeded | Code::Unknown | Code::Internal
        )
    }

    fn log_transition(from: State, to: State, reason: &str) {
        if from != to {
            tracing::warn!(
                circuit_breaker_from = ?from,
                circuit_breaker_to = ?to,
                reason = reason,
                "circuit breaker transition"
            );
        }
    }

    fn reject_open<T>() -> Result<T, Status> {
        Err(Status::unavailable(
            "circuit breaker open: flight service calls are blocked",
        ))
    }

    async fn acquire_permission(&self) -> Result<Permission, Status> {
        let mut inner = self.inner.lock().await;
        let now = Instant::now();

        match inner.state {
            State::Closed => Ok(Permission::Closed),
            State::Open => {
                let opened_at = inner.opened_at.unwrap_or(now);
                if now.duration_since(opened_at) >= self.cfg.open_timeout {
                    let from = inner.state;
                    inner.state = State::HalfOpen;
                    inner.half_open_probe_in_flight = false;
                    Self::log_transition(from, inner.state, "open timeout elapsed");
                    // fallthrough into HALF_OPEN
                } else {
                    return Self::reject_open();
                }
                // continue
                if inner.half_open_probe_in_flight {
                    Self::reject_open()
                } else {
                    inner.half_open_probe_in_flight = true;
                    Ok(Permission::HalfOpenProbe)
                }
            }
            State::HalfOpen => {
                if inner.half_open_probe_in_flight {
                    Self::reject_open()
                } else {
                    inner.half_open_probe_in_flight = true;
                    Ok(Permission::HalfOpenProbe)
                }
            }
        }
    }

    async fn on_success(&self, permission: Permission) {
        let mut inner = self.inner.lock().await;
        match permission {
            Permission::Closed => {
                inner.consecutive_failures = 0;
                inner.window.push_back(false);
                while inner.window.len() > self.cfg.window_size {
                    inner.window.pop_front();
                }
            }
            Permission::HalfOpenProbe => {
                inner.half_open_probe_in_flight = false;
                let from = inner.state;
                inner.state = State::Closed;
                inner.opened_at = None;
                inner.window.clear();
                inner.consecutive_failures = 0;
                Self::log_transition(from, inner.state, "half-open probe succeeded");
            }
        }
    }

    async fn on_failure(&self, permission: Permission) {
        let mut inner = self.inner.lock().await;
        match permission {
            Permission::Closed => {
                inner.consecutive_failures = inner.consecutive_failures.saturating_add(1);
                inner.window.push_back(true);
                while inner.window.len() > self.cfg.window_size {
                    inner.window.pop_front();
                }

                let should_open = match self.cfg.window_mode {
                    WindowMode::Consecutive => inner.consecutive_failures >= self.cfg.error_threshold,
                    WindowMode::Rolling => inner.window.iter().filter(|v| **v).count()
                        >= self.cfg.error_threshold,
                };

                if should_open {
                    let from = inner.state;
                    inner.state = State::Open;
                    inner.opened_at = Some(Instant::now());
                    inner.half_open_probe_in_flight = false;
                    Self::log_transition(from, inner.state, "error threshold reached");
                }
            }
            Permission::HalfOpenProbe => {
                inner.half_open_probe_in_flight = false;
                let from = inner.state;
                inner.state = State::Open;
                inner.opened_at = Some(Instant::now());
                inner.window.clear();
                inner.consecutive_failures = 0;
                Self::log_transition(from, inner.state, "half-open probe failed");
            }
        }
    }

    pub async fn call<T, F, Fut>(&self, f: F) -> Result<T, Status>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<T, Status>>,
    {
        let permission = self.acquire_permission().await?;

        let result = f().await;
        match &result {
            Ok(_) => self.on_success(permission).await,
            Err(status) => {
                if Self::should_count_failure(status.code()) {
                    self.on_failure(permission).await;
                } else {
                    // Business errors should not poison the breaker.
                    self.on_success(permission).await;
                }
            }
        }
        result
    }
}

#[derive(Debug, Clone, Copy)]
enum Permission {
    Closed,
    HalfOpenProbe,
}

