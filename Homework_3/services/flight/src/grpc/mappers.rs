use prost_types::Timestamp;
use tonic::Status;
use uuid::Uuid;

use crate::flight::v1::{Flight, FlightStatus, ReservationStatus, SeatReservation};
use crate::models::{FlightModel, SeatReservationModel};

pub fn parse_uuid(value: &str, field_name: &str) -> Result<Uuid, Status> {
    Uuid::parse_str(value).map_err(|_| Status::invalid_argument(format!("invalid {field_name}")))
}

fn to_timestamp(value: chrono::DateTime<chrono::Utc>) -> Timestamp {
    Timestamp {
        seconds: value.timestamp(),
        nanos: value.timestamp_subsec_nanos() as i32,
    }
}

fn map_flight_status(status: &str) -> FlightStatus {
    match status {
        "SCHEDULED" => FlightStatus::Scheduled,
        "DEPARTED" => FlightStatus::Departed,
        "CANCELLED" => FlightStatus::Cancelled,
        "COMPLETED" => FlightStatus::Completed,
        _ => FlightStatus::Unspecified,
    }
}

fn map_reservation_status(status: &str) -> ReservationStatus {
    match status {
        "ACTIVE" => ReservationStatus::Active,
        "RELEASED" => ReservationStatus::Released,
        "EXPIRED" => ReservationStatus::Expired,
        _ => ReservationStatus::Unspecified,
    }
}

pub fn map_flight(model: &FlightModel) -> Result<Flight, Status> {
    let total_seats =
        u32::try_from(model.total_seats).map_err(|_| Status::internal("negative total_seats"))?;
    let available_seats = u32::try_from(model.available_seats)
        .map_err(|_| Status::internal("negative available_seats"))?;
    let price_minor_units = u64::try_from(model.price_minor_units)
        .map_err(|_| Status::internal("negative price_minor_units"))?;

    Ok(Flight {
        id: model.id.to_string(),
        flight_number: model.flight_number.clone(),
        airline: model.airline.clone(),
        origin: model.origin.clone(),
        destination: model.destination.clone(),
        departure_time: Some(to_timestamp(model.departure_time)),
        arrival_time: Some(to_timestamp(model.arrival_time)),
        total_seats,
        available_seats,
        price_currency: model.price_currency.clone(),
        price_minor_units,
        status: map_flight_status(&model.status) as i32,
        created_at: Some(to_timestamp(model.created_at)),
        updated_at: Some(to_timestamp(model.updated_at)),
    })
}

pub fn map_reservation(model: &SeatReservationModel) -> Result<SeatReservation, Status> {
    let seat_count =
        u32::try_from(model.seat_count).map_err(|_| Status::internal("negative seat_count"))?;

    Ok(SeatReservation {
        id: model.id.to_string(),
        booking_id: model.booking_id.to_string(),
        flight_id: model.flight_id.to_string(),
        seat_count,
        status: map_reservation_status(&model.status) as i32,
        created_at: Some(to_timestamp(model.created_at)),
        updated_at: Some(to_timestamp(model.updated_at)),
    })
}
