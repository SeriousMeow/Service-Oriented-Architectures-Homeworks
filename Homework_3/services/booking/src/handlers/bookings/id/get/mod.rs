use crate::api;
use crate::db::Repository;
use crate::handlers::common;
use crate::state::AppState;

pub async fn handle(
    state: &AppState,
    request: api::GetBookingsByIdRequest,
) -> anyhow::Result<api::GetBookingsByIdResponse> {
    let client = state.db.get().await?;
    let booking = client.get_booking_by_id(request.path.id).await?;

    let Some(booking) = booking else {
        return Ok(api::GetBookingsByIdResponse::NotFound(common::error(
            "BOOKING_NOT_FOUND",
            "booking not found",
        )));
    };

    Ok(api::GetBookingsByIdResponse::Ok(common::map_booking(
        &booking,
    )?))
}
