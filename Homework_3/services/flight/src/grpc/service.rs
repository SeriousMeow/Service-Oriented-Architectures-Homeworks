use chrono::{DateTime, Utc};
use deadpool_redis::redis::AsyncCommands;
use prost::Message;
use tonic::{Request, Response, Status};

use crate::db::Repository;
use crate::flight::v1::flight_service_server::FlightService;
use crate::flight::v1::{
    GetFlightRequest, GetFlightResponse, ReleaseReservationRequest, ReleaseReservationResponse,
    ReserveSeatsRequest, ReserveSeatsResponse, SearchFlightsRequest, SearchFlightsResponse,
};
use crate::grpc::mappers::{map_flight, map_reservation, parse_uuid};
use crate::models::FlightStatusDb;
use crate::state::{AppState, RedisConnection};

#[derive(Clone)]
pub struct FlightGrpcService {
    state: AppState,
}

impl FlightGrpcService {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }

    fn flight_cache_key(flight_id: uuid::Uuid) -> String {
        format!("flight:{flight_id}")
    }

    fn search_cache_key(
        origin: &str,
        destination: &str,
        departure_date: Option<chrono::NaiveDate>,
    ) -> String {
        let date_key = match departure_date {
            Some(d) => d.to_string(),
            None => "no-date".to_string(),
        };
        format!("search:{origin}:{destination}:{date_key}")
    }

    fn flight_search_index_key(flight_id: &str) -> String {
        format!("flight_search_keys:{flight_id}")
    }

    async fn cache_set_search_index(
        &self,
        redis: &mut RedisConnection,
        search_key: &str,
        response: &SearchFlightsResponse,
    ) {
        for flight in &response.flights {
            if flight.id.trim().is_empty() {
                continue;
            }
            let index_key = Self::flight_search_index_key(&flight.id);
            if let Err(e) = redis.sadd::<_, _, ()>(&index_key, search_key).await {
                tracing::warn!(
                    "cache error: failed to add search key to index {}: {}",
                    index_key,
                    e
                );
            }
        }
    }

    async fn invalidate_flight_cache(&self, flight_id: uuid::Uuid) {
        let Ok(mut redis) = self.state.redis.get().await else {
            tracing::warn!("cache error: failed to get redis connection for invalidation");
            return;
        };

        let flight_key = Self::flight_cache_key(flight_id);
        if let Err(e) = redis.del::<_, ()>(&flight_key).await {
            tracing::warn!(
                "cache error: failed to invalidate flight key {}: {}",
                flight_key,
                e
            );
        }

        let index_key = Self::flight_search_index_key(&flight_id.to_string());
        let search_keys: Result<Vec<String>, _> = redis.smembers(&index_key).await;
        match search_keys {
            Ok(keys) => {
                let mut deleted = 0usize;
                for key in &keys {
                    if let Err(e) = redis.del::<_, ()>(key).await {
                        tracing::warn!(
                            "cache error: failed to invalidate search key {}: {}",
                            key,
                            e
                        );
                    } else {
                        deleted += 1;
                    }
                }

                if let Err(e) = redis.del::<_, ()>(&index_key).await {
                    tracing::warn!(
                        "cache error: failed to delete search index {}: {}",
                        index_key,
                        e
                    );
                }

                tracing::info!(
                    "cache invalidate: flight_id={} (flight key + {} search keys)",
                    flight_id,
                    deleted
                );
            }
            Err(e) => {
                tracing::warn!(
                    "cache error: failed to read search index {}: {}",
                    index_key,
                    e
                );
            }
        }
    }
}

#[tonic::async_trait]
impl FlightService for FlightGrpcService {
    async fn search_flights(
        &self,
        request: Request<SearchFlightsRequest>,
    ) -> Result<Response<SearchFlightsResponse>, Status> {
        let req = request.into_inner();

        if req.origin.trim().is_empty() || req.destination.trim().is_empty() {
            return Err(Status::invalid_argument(
                "origin and destination are required",
            ));
        }

        let origin = req.origin.trim().to_uppercase();
        let destination = req.destination.trim().to_uppercase();
        let departure_date = match req.departure_date.as_ref() {
            Some(date) => Some(
                DateTime::<Utc>::from_timestamp(date.seconds, date.nanos as u32)
                    .ok_or_else(|| Status::invalid_argument("invalid departure_date"))?
                    .date_naive(),
            ),
            None => None,
        };

        let cache_key = Self::search_cache_key(&origin, &destination, departure_date);
        if let Ok(mut redis) = self.state.redis.get().await {
            let cached: Result<Option<Vec<u8>>, _> = redis.get(&cache_key).await;
            match cached {
                Ok(Some(bytes)) => match SearchFlightsResponse::decode(bytes.as_slice()) {
                    Ok(resp) => {
                        tracing::info!("cache hit: {}", cache_key);
                        return Ok(Response::new(resp));
                    }
                    Err(e) => {
                        tracing::warn!("cache error: failed to decode {}: {}", cache_key, e);
                    }
                },
                Ok(None) => tracing::info!("cache miss: {}", cache_key),
                Err(e) => tracing::warn!("cache error: failed to GET {}: {}", cache_key, e),
            }
        } else {
            tracing::warn!(
                "cache error: failed to get redis connection for {}",
                cache_key
            );
        }

        let client = self
            .state
            .db
            .get()
            .await
            .map_err(|e| Status::unavailable(format!("failed to get db connection: {e}")))?;

        let flights = client
            .search_flights(&origin, &destination, departure_date)
            .await
            .map_err(|e| Status::internal(format!("db error: {e}")))?
            .iter()
            .map(map_flight)
            .collect::<Result<Vec<_>, _>>()?;

        let response = SearchFlightsResponse { flights };
        if let Ok(mut redis) = self.state.redis.get().await {
            let ttl: u64 = self.state.cache_ttl_seconds;
            let bytes = response.encode_to_vec();
            if let Err(e) = redis.set_ex::<_, _, ()>(&cache_key, bytes, ttl).await {
                tracing::warn!("cache error: failed to SETEX {}: {}", cache_key, e);
            } else {
                tracing::info!("cache set: {} ttl={}s", cache_key, ttl);
                self.cache_set_search_index(&mut redis, &cache_key, &response)
                    .await;
            }
        } else {
            tracing::warn!(
                "cache error: failed to get redis connection for SET {}",
                cache_key
            );
        }

        Ok(Response::new(response))
    }

    async fn get_flight(
        &self,
        request: Request<GetFlightRequest>,
    ) -> Result<Response<GetFlightResponse>, Status> {
        let req = request.into_inner();
        let flight_id = parse_uuid(&req.flight_id, "flight_id")?;

        let cache_key = Self::flight_cache_key(flight_id);
        if let Ok(mut redis) = self.state.redis.get().await {
            let cached: Result<Option<Vec<u8>>, _> = redis.get(&cache_key).await;
            match cached {
                Ok(Some(bytes)) => match GetFlightResponse::decode(bytes.as_slice()) {
                    Ok(resp) => {
                        tracing::info!("cache hit: {}", cache_key);
                        return Ok(Response::new(resp));
                    }
                    Err(e) => {
                        tracing::warn!("cache error: failed to decode {}: {}", cache_key, e);
                    }
                },
                Ok(None) => tracing::info!("cache miss: {}", cache_key),
                Err(e) => tracing::warn!("cache error: failed to GET {}: {}", cache_key, e),
            }
        } else {
            tracing::warn!(
                "cache error: failed to get redis connection for {}",
                cache_key
            );
        }

        let client = self
            .state
            .db
            .get()
            .await
            .map_err(|e| Status::unavailable(format!("failed to get db connection: {e}")))?;

        let flight = client
            .get_flight_by_id(flight_id)
            .await
            .map_err(|e| Status::internal(format!("db error: {e}")))?
            .ok_or_else(|| Status::not_found("flight not found"))?;

        let response = GetFlightResponse {
            flight: Some(map_flight(&flight)?),
        };

        if let Ok(mut redis) = self.state.redis.get().await {
            let ttl: u64 = self.state.cache_ttl_seconds;
            let bytes = response.encode_to_vec();
            if let Err(e) = redis.set_ex::<_, _, ()>(&cache_key, bytes, ttl).await {
                tracing::warn!("cache error: failed to SETEX {}: {}", cache_key, e);
            } else {
                tracing::info!("cache set: {} ttl={}s", cache_key, ttl);
            }
        } else {
            tracing::warn!(
                "cache error: failed to get redis connection for SET {}",
                cache_key
            );
        }

        Ok(Response::new(response))
    }

    async fn reserve_seats(
        &self,
        request: Request<ReserveSeatsRequest>,
    ) -> Result<Response<ReserveSeatsResponse>, Status> {
        let req = request.into_inner();
        let booking_id = parse_uuid(&req.booking_id, "booking_id")?;
        let flight_id = parse_uuid(&req.flight_id, "flight_id")?;

        if req.seat_count == 0 {
            return Err(Status::invalid_argument("seat_count must be > 0"));
        }
        let seat_count = i32::try_from(req.seat_count)
            .map_err(|_| Status::invalid_argument("seat_count is too large"))?;

        let mut client = self
            .state
            .db
            .get()
            .await
            .map_err(|e| Status::unavailable(format!("failed to get db connection: {e}")))?;
        let tx = client
            .transaction()
            .await
            .map_err(|e| Status::internal(format!("db begin tx error: {e}")))?;

        if let Some(existing_reservation) = tx
            .get_reservation_by_booking_id(booking_id)
            .await
            .map_err(|e| Status::internal(format!("db error: {e}")))?
        {
            if existing_reservation.flight_id != flight_id {
                return Err(Status::failed_precondition(
                    "booking_id is already linked to another flight",
                ));
            }

            let existing_flight = tx
                .get_flight_by_id(existing_reservation.flight_id)
                .await
                .map_err(|e| Status::internal(format!("db error: {e}")))?
                .ok_or_else(|| Status::internal("reservation exists but flight not found"))?;

            tx.commit()
                .await
                .map_err(|e| Status::internal(format!("db commit error: {e}")))?;

            return Ok(Response::new(ReserveSeatsResponse {
                flight: Some(map_flight(&existing_flight)?),
                reservation: Some(map_reservation(&existing_reservation)?),
            }));
        }

        let (status, available_seats) = tx
            .lock_flight_status_and_available_for_update(flight_id)
            .await
            .map_err(|e| Status::internal(format!("db error: {e}")))?
            .ok_or_else(|| Status::not_found("flight not found"))?;

        if status != FlightStatusDb::Scheduled {
            return Err(Status::failed_precondition(
                "flight is not in SCHEDULED status",
            ));
        }

        if available_seats < seat_count {
            return Err(Status::resource_exhausted("not enough seats"));
        }

        let reserved_flight = tx
            .try_reserve_seats_on_scheduled_flight(flight_id, seat_count)
            .await
            .map_err(|e| Status::internal(format!("db error: {e}")))?;

        let Some(reserved_flight) = reserved_flight else {
            return Err(Status::internal(
                "flight row was locked and validated but seat reservation update returned no row",
            ));
        };

        let reservation = tx
            .create_active_reservation(booking_id, flight_id, seat_count)
            .await
            .map_err(|e| Status::internal(format!("db error: {e}")))?;

        tx.commit()
            .await
            .map_err(|e| Status::internal(format!("db commit error: {e}")))?;

        self.invalidate_flight_cache(flight_id).await;

        Ok(Response::new(ReserveSeatsResponse {
            flight: Some(map_flight(&reserved_flight)?),
            reservation: Some(map_reservation(&reservation)?),
        }))
    }

    async fn release_reservation(
        &self,
        request: Request<ReleaseReservationRequest>,
    ) -> Result<Response<ReleaseReservationResponse>, Status> {
        let req = request.into_inner();
        let booking_id = parse_uuid(&req.booking_id, "booking_id")?;

        let mut client = self
            .state
            .db
            .get()
            .await
            .map_err(|e| Status::unavailable(format!("failed to get db connection: {e}")))?;
        let tx = client
            .transaction()
            .await
            .map_err(|e| Status::internal(format!("db begin tx error: {e}")))?;

        let reservation = tx
            .get_reservation_by_booking_id_for_update(booking_id)
            .await
            .map_err(|e| Status::internal(format!("db error: {e}")))?
            .ok_or_else(|| Status::not_found("reservation not found"))?;

        if reservation.status != "ACTIVE" {
            return Err(Status::failed_precondition("reservation is not active"));
        }

        let flight = tx
            .release_flight_seats(reservation.flight_id, reservation.seat_count)
            .await
            .map_err(|e| Status::internal(format!("db error: {e}")))?
            .ok_or_else(|| Status::internal("reservation flight not found"))?;

        let reservation = tx
            .mark_reservation_released(reservation.id)
            .await
            .map_err(|e| Status::internal(format!("db error: {e}")))?;

        tx.commit()
            .await
            .map_err(|e| Status::internal(format!("db commit error: {e}")))?;

        self.invalidate_flight_cache(reservation.flight_id).await;

        Ok(Response::new(ReleaseReservationResponse {
            flight: Some(map_flight(&flight)?),
            reservation: Some(map_reservation(&reservation)?),
        }))
    }
}
