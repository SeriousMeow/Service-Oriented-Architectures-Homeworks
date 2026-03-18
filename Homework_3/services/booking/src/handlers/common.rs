use chrono::{DateTime, Utc};
use tonic::{Code, Status};

use crate::api;
use crate::flight::v1::{Flight, FlightStatus as GrpcFlightStatus};
use crate::models::{BookingModel, BookingStatus};

pub fn error(code: &str, message: impl Into<String>) -> api::ErrorResponse {
    api::ErrorResponse {
        code: code.to_string(),
        message: message.into(),
    }
}

pub fn map_booking(model: &BookingModel) -> anyhow::Result<api::Booking> {
    Ok(api::Booking {
        id: model.id,
        user_id: model.user_id,
        flight_id: model.flight_id,
        passenger_name: model.passenger_name.clone(),
        passenger_email: model.passenger_email.clone(),
        seat_count: i64::from(model.seat_count),
        total_price: api::Money {
            currency: model.price_currency.clone(),
            amount_minor: model.total_price_minor,
        },
        status: map_booking_status(model.status)?,
        created_at: model.created_at,
        updated_at: model.updated_at,
    })
}

pub fn map_flight(flight: &Flight) -> anyhow::Result<api::Flight> {
    let departure_time = map_timestamp(flight.departure_time.as_ref())?;
    let arrival_time = map_timestamp(flight.arrival_time.as_ref())?;

    Ok(api::Flight {
        id: uuid::Uuid::parse_str(&flight.id)?,
        flight_number: flight.flight_number.clone(),
        airline: flight.airline.clone(),
        origin: flight.origin.clone(),
        destination: flight.destination.clone(),
        departure_time,
        arrival_time,
        total_seats: i64::from(flight.total_seats),
        available_seats: i64::from(flight.available_seats),
        price: api::Money {
            currency: flight.price_currency.clone(),
            amount_minor: i64::try_from(flight.price_minor_units)?,
        },
        status: map_flight_status(flight.status)?,
    })
}

pub fn grpc_message(status: &Status) -> String {
    if status.message().is_empty() {
        status.code().to_string()
    } else {
        status.message().to_string()
    }
}

pub fn grpc_upstream_error(status: &Status) -> api::ErrorResponse {
    error("FLIGHT_SERVICE_ERROR", grpc_message(status))
}

pub fn is_circuit_open(status: &Status) -> bool {
    status.code() == Code::Unavailable
        && status
            .message()
            .to_lowercase()
            .contains("circuit breaker open")
}

fn map_timestamp(timestamp: Option<&prost_types::Timestamp>) -> anyhow::Result<DateTime<Utc>> {
    let value = timestamp.ok_or_else(|| anyhow::anyhow!("missing timestamp"))?;
    DateTime::<Utc>::from_timestamp(value.seconds, value.nanos as u32)
        .ok_or_else(|| anyhow::anyhow!("invalid timestamp"))
}

fn map_flight_status(status: i32) -> anyhow::Result<api::FlightStatus> {
    let grpc_status =
        GrpcFlightStatus::try_from(status).map_err(|_| anyhow::anyhow!("unknown flight status"))?;
    match grpc_status {
        GrpcFlightStatus::Scheduled => Ok(api::FlightStatus::Scheduled),
        GrpcFlightStatus::Departed => Ok(api::FlightStatus::Departed),
        GrpcFlightStatus::Cancelled => Ok(api::FlightStatus::Cancelled),
        GrpcFlightStatus::Completed => Ok(api::FlightStatus::Completed),
        GrpcFlightStatus::Unspecified => Err(anyhow::anyhow!("unspecified flight status")),
    }
}

fn map_booking_status(status: BookingStatus) -> anyhow::Result<api::BookingStatus> {
    Ok(match status {
        BookingStatus::Confirmed => api::BookingStatus::Confirmed,
        BookingStatus::Cancelled => api::BookingStatus::Cancelled,
    })
}
