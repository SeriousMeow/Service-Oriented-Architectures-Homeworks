use std::str::FromStr;
use std::time::Duration;

use tonic::metadata::{Ascii, MetadataValue};
use tonic::transport::{Channel, Endpoint};
use tonic::{Code, Request, Status};
use uuid::Uuid;

use crate::flight::v1::flight_service_client::FlightServiceClient;
use crate::flight::v1::{
    GetFlightRequest, GetFlightResponse, ReleaseReservationRequest, ReleaseReservationResponse,
    ReserveSeatsRequest, ReserveSeatsResponse, SearchFlightsRequest, SearchFlightsResponse,
};
use crate::grpc::circuit_breaker::{CircuitBreaker, CircuitBreakerConfig};

const MAX_RETRIES: usize = 3;
const BACKOFF_BASE_MS: u64 = 100;

#[derive(Clone)]
pub struct FlightClientConfig {
    endpoint: Endpoint,
    api_key: Option<MetadataValue<Ascii>>,
    circuit_breaker: CircuitBreaker,
}

impl FlightClientConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        let addr =
            std::env::var("FLIGHT_SERVICE_ADDR").unwrap_or_else(|_| "localhost:50051".to_string());
        let endpoint = Endpoint::from_shared(format!("http://{addr}"))?
            // Avoid hanging HTTP handlers on DNS/TCP issues.
            .connect_timeout(Duration::from_secs(2))
            .timeout(Duration::from_secs(5));
        let api_key = std::env::var("FLIGHT_SERVICE_API_KEY")
            .ok()
            .filter(|value| !value.is_empty())
            .map(|value| MetadataValue::from_str(&value))
            .transpose()?;
        let circuit_breaker = CircuitBreaker::new(CircuitBreakerConfig::from_env());
        Ok(Self {
            endpoint,
            api_key,
            circuit_breaker,
        })
    }

    async fn client(&self) -> Result<FlightServiceClient<Channel>, Status> {
        let channel = self.endpoint.connect().await.map_err(|error| {
            Status::new(
                Code::Unavailable,
                format!("flight service unavailable: {error}"),
            )
        })?;
        Ok(FlightServiceClient::new(channel))
    }

    fn with_auth<T>(&self, payload: T) -> Request<T> {
        let mut request = Request::new(payload);
        if let Some(api_key) = self.api_key.as_ref() {
            request
                .metadata_mut()
                .insert("x-service-api-key", api_key.clone());
        }
        request
    }

    fn is_retryable(code: Code) -> bool {
        matches!(code, Code::Unavailable | Code::DeadlineExceeded)
    }

    fn backoff_duration(retry_idx: usize) -> Duration {
        // retry_idx: 1 for first retry, 2 for second, ...
        // With MAX_RETRIES=3 this yields sleeps of 100ms, 200ms, 400ms.
        let shift = retry_idx.saturating_sub(1) as u32;
        let factor = 1u64.checked_shl(shift).unwrap_or(u64::MAX);
        Duration::from_millis(BACKOFF_BASE_MS.saturating_mul(factor))
    }

    async fn call_with_retry<T, F, Fut>(&self, mut f: F) -> Result<T, Status>
    where
        F: FnMut(FlightServiceClient<Channel>) -> Fut,
        Fut: std::future::Future<Output = Result<T, Status>>,
    {
        let mut last_err: Option<Status> = None;

        // Attempt 0 is the initial call, attempts 1..=MAX_RETRIES are retries.
        for attempt in 0..=MAX_RETRIES {
            if attempt > 0 {
                tokio::time::sleep(Self::backoff_duration(attempt)).await;
            }

            let client = match self.client().await {
                Ok(client) => client,
                Err(status) => {
                    last_err = Some(status);
                    if attempt >= MAX_RETRIES
                        || !Self::is_retryable(last_err.as_ref().unwrap().code())
                    {
                        break;
                    }
                    continue;
                }
            };

            match f(client).await {
                Ok(value) => return Ok(value),
                Err(status) => {
                    let code = status.code();
                    last_err = Some(status);

                    if attempt >= MAX_RETRIES || !Self::is_retryable(code) {
                        break;
                    }
                }
            }
        }

        Err(last_err.unwrap_or_else(|| {
            Status::new(Code::Unknown, "flight service call failed without status")
        }))
    }

    pub async fn search_flights(
        &self,
        origin: String,
        destination: String,
        departure_date: Option<prost_types::Timestamp>,
    ) -> Result<SearchFlightsResponse, Status> {
        self.circuit_breaker
            .call(|| async {
                self.call_with_retry(|mut client| {
                    let request = self.with_auth(SearchFlightsRequest {
                        origin: origin.clone(),
                        destination: destination.clone(),
                        departure_date: departure_date.clone(),
                    });
                    async move { client.search_flights(request).await.map(|r| r.into_inner()) }
                })
                .await
            })
            .await
    }

    pub async fn get_flight(&self, flight_id: Uuid) -> Result<GetFlightResponse, Status> {
        self.circuit_breaker
            .call(|| async {
                self.call_with_retry(|mut client| {
                    let request = self.with_auth(GetFlightRequest {
                        flight_id: flight_id.to_string(),
                    });
                    async move { client.get_flight(request).await.map(|r| r.into_inner()) }
                })
                .await
            })
            .await
    }

    pub async fn reserve_seats(
        &self,
        booking_id: Uuid,
        flight_id: Uuid,
        seat_count: u32,
    ) -> Result<ReserveSeatsResponse, Status> {
        self.circuit_breaker
            .call(|| async {
                self.call_with_retry(|mut client| {
                    let request = self.with_auth(ReserveSeatsRequest {
                        booking_id: booking_id.to_string(),
                        flight_id: flight_id.to_string(),
                        seat_count,
                    });
                    async move { client.reserve_seats(request).await.map(|r| r.into_inner()) }
                })
                .await
            })
            .await
    }

    pub async fn release_reservation(
        &self,
        booking_id: Uuid,
    ) -> Result<ReleaseReservationResponse, Status> {
        self.circuit_breaker
            .call(|| async {
                self.call_with_retry(|mut client| {
                    let request = self.with_auth(ReleaseReservationRequest {
                        booking_id: booking_id.to_string(),
                    });
                    async move {
                        client
                            .release_reservation(request)
                            .await
                            .map(|r| r.into_inner())
                    }
                })
                .await
            })
            .await
    }
}
