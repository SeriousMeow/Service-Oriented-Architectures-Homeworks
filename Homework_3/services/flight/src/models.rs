use chrono::{DateTime, Utc};
use postgres_types::{FromSql, ToSql};
use tokio_postgres::Row;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, ToSql, FromSql)]
#[postgres(name = "flight_status")]
pub enum FlightStatusDb {
    #[postgres(name = "SCHEDULED")]
    Scheduled,
    #[postgres(name = "DEPARTED")]
    Departed,
    #[postgres(name = "CANCELLED")]
    Cancelled,
    #[postgres(name = "COMPLETED")]
    Completed,
}

#[derive(Debug, Clone)]
pub struct FlightModel {
    pub id: Uuid,
    pub flight_number: String,
    pub airline: String,
    pub origin: String,
    pub destination: String,
    pub departure_time: DateTime<Utc>,
    pub arrival_time: DateTime<Utc>,
    pub total_seats: i32,
    pub available_seats: i32,
    pub price_currency: String,
    pub price_minor_units: i64,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl FlightModel {
    pub fn from_row(row: &Row) -> Self {
        Self {
            id: row.get("id"),
            flight_number: row.get("flight_number"),
            airline: row.get("airline"),
            origin: row.get::<_, String>("origin").trim().to_string(),
            destination: row.get::<_, String>("destination").trim().to_string(),
            departure_time: row.get("departure_time"),
            arrival_time: row.get("arrival_time"),
            total_seats: row.get("total_seats"),
            available_seats: row.get("available_seats"),
            price_currency: row.get::<_, String>("price_currency").trim().to_string(),
            price_minor_units: row.get("price_minor_units"),
            status: row.get("status"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SeatReservationModel {
    pub id: Uuid,
    pub booking_id: Uuid,
    pub flight_id: Uuid,
    pub seat_count: i32,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl SeatReservationModel {
    pub fn from_row(row: &Row) -> Self {
        Self {
            id: row.get("id"),
            booking_id: row.get("booking_id"),
            flight_id: row.get("flight_id"),
            seat_count: row.get("seat_count"),
            status: row.get("status"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        }
    }
}
