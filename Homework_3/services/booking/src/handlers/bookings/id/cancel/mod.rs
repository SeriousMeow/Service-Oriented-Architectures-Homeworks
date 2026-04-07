use tonic::Code;

use crate::api;
use crate::db::Repository;
use crate::handlers::common;
use crate::models::BookingStatus;
use crate::state::AppState;

pub async fn handle(
    state: &AppState,
    request: api::PostBookingsByIdCancelRequest,
) -> anyhow::Result<api::PostBookingsByIdCancelResponse> {
    let client = state.db.get().await?;
    let booking = client.get_booking_by_id(request.path.id).await?;

    let Some(booking) = booking else {
        return Ok(api::PostBookingsByIdCancelResponse::NotFound(
            common::error("BOOKING_NOT_FOUND", "booking not found"),
        ));
    };

    if booking.status != BookingStatus::Confirmed {
        return Ok(api::PostBookingsByIdCancelResponse::Conflict(
            common::error(
                "BOOKING_CANNOT_BE_CANCELLED",
                "booking is not in CONFIRMED status",
            ),
        ));
    }

    match state.flight_client.release_reservation(booking.id).await {
        Ok(_) => {}
        Err(status) if common::is_circuit_open(&status) => {
            return Ok(api::PostBookingsByIdCancelResponse::ServiceUnavailable(
                common::error("FLIGHT_SERVICE_UNAVAILABLE", common::grpc_message(&status)),
            ));
        }
        Err(status) if status.code() == Code::NotFound => {
            return Ok(api::PostBookingsByIdCancelResponse::NotFound(
                common::error("RESERVATION_NOT_FOUND", common::grpc_message(&status)),
            ));
        }
        Err(status) if status.code() == Code::FailedPrecondition => {
            return Ok(api::PostBookingsByIdCancelResponse::Conflict(
                common::error("RESERVATION_NOT_ACTIVE", common::grpc_message(&status)),
            ));
        }
        Err(status) => {
            return Ok(api::PostBookingsByIdCancelResponse::BadGateway(
                common::grpc_upstream_error(&status),
            ));
        }
    }

    let updated = client
        .update_booking_status(booking.id, BookingStatus::Cancelled)
        .await?;

    let Some(updated) = updated else {
        return Ok(api::PostBookingsByIdCancelResponse::NotFound(
            common::error("BOOKING_NOT_FOUND", "booking not found"),
        ));
    };

    Ok(api::PostBookingsByIdCancelResponse::Ok(
        common::map_booking(&updated)?,
    ))
}
