use tonic::Code;
use uuid::Uuid;

use crate::api;
use crate::db::Repository;
use crate::handlers::common;
use crate::state::AppState;

pub async fn handle(
    state: &AppState,
    request: api::PostBookingsRequest,
) -> anyhow::Result<api::PostBookingsResponse> {
    let booking_id = Uuid::new_v4();
    let seat_count = u32::try_from(request.body.seat_count)
        .map_err(|_| anyhow::anyhow!("seat_count must be positive"))?;

    let flight_response = state.flight_client.get_flight(request.body.flight_id).await;
    let flight_response = match flight_response {
        Ok(value) => value,
        Err(status) if common::is_circuit_open(&status) => {
            return Ok(api::PostBookingsResponse::ServiceUnavailable(common::error(
                "FLIGHT_SERVICE_UNAVAILABLE",
                common::grpc_message(&status),
            )));
        }
        Err(status) if status.code() == Code::NotFound => {
            return Ok(api::PostBookingsResponse::NotFound(common::error(
                "FLIGHT_NOT_FOUND",
                common::grpc_message(&status),
            )));
        }
        Err(status) if status.code() == Code::InvalidArgument => {
            return Ok(api::PostBookingsResponse::BadRequest(common::error(
                "INVALID_ARGUMENT",
                common::grpc_message(&status),
            )));
        }
        Err(status) => {
            return Ok(api::PostBookingsResponse::BadGateway(
                common::grpc_upstream_error(&status),
            ));
        }
    };

    let flight = flight_response
        .flight
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("flight payload is missing"))?;

    let total_price_minor = i64::try_from(flight.price_minor_units)?
        .checked_mul(i64::from(request.body.seat_count))
        .ok_or_else(|| anyhow::anyhow!("price overflow"))?;

    let reserve_response = state
        .flight_client
        .reserve_seats(booking_id, request.body.flight_id, seat_count)
        .await;

    if let Err(status) = reserve_response {
        return Ok(match status.code() {
            Code::ResourceExhausted => api::PostBookingsResponse::Conflict(common::error(
                "NOT_ENOUGH_SEATS",
                common::grpc_message(&status),
            )),
            Code::NotFound => api::PostBookingsResponse::NotFound(common::error(
                "FLIGHT_NOT_FOUND",
                common::grpc_message(&status),
            )),
            Code::InvalidArgument => api::PostBookingsResponse::BadRequest(common::error(
                "INVALID_ARGUMENT",
                common::grpc_message(&status),
            )),
            _ if common::is_circuit_open(&status) => api::PostBookingsResponse::ServiceUnavailable(
                common::error("FLIGHT_SERVICE_UNAVAILABLE", common::grpc_message(&status)),
            ),
            _ => api::PostBookingsResponse::BadGateway(common::grpc_upstream_error(&status)),
        });
    }

    let client = state.db.get().await?;
    let booking = client
        .create_booking(
            booking_id,
            request.body.user_id,
            request.body.flight_id,
            &request.body.passenger_name,
            &request.body.passenger_email,
            i32::try_from(request.body.seat_count)?,
            &flight.price_currency,
            total_price_minor,
        )
        .await?;

    Ok(api::PostBookingsResponse::Created(common::map_booking(
        &booking,
    )?))
}
