use crate::api;
use crate::db::Repository;
use crate::handlers::common;
use crate::state::AppState;

pub async fn handle(
    state: &AppState,
    request: api::GetBookingsRequest,
) -> anyhow::Result<api::GetBookingsResponse> {
    let client = state.db.get().await?;
    let bookings = client.list_bookings_by_user(request.query.user_id).await?;

    let mut items = Vec::with_capacity(bookings.len());
    for booking in &bookings {
        items.push(common::map_booking(booking)?);
    }

    Ok(api::GetBookingsResponse::Ok(api::BookingListResponse {
        items,
    }))
}
