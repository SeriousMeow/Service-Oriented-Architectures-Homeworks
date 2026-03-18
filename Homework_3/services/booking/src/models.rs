use chrono::{DateTime, Utc};
use tokio_postgres::Row;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct BookingModel {
    pub id: Uuid,
    pub user_id: Uuid,
    pub flight_id: Uuid,
    pub passenger_name: String,
    pub passenger_email: String,
    pub seat_count: i32,
    pub price_currency: String,
    pub total_price_minor: i64,
    pub status: BookingStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl BookingModel {
    pub fn from_row(row: &Row) -> Self {
        Self {
            id: row.get("id"),
            user_id: row.get("user_id"),
            flight_id: row.get("flight_id"),
            passenger_name: row.get("passenger_name"),
            passenger_email: row.get("passenger_email"),
            seat_count: row.get("seat_count"),
            price_currency: row.get::<_, String>("price_currency").trim().to_string(),
            total_price_minor: row.get("total_price_minor"),
            status: row.get("status"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BookingStatus {
    Confirmed,
    Cancelled,
}

impl BookingStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Confirmed => "CONFIRMED",
            Self::Cancelled => "CANCELLED",
        }
    }
}
