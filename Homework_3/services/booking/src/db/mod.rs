use async_trait::async_trait;
use deadpool_postgres::{GenericClient, Object, Transaction};
use tokio_postgres::Error;
use uuid::Uuid;

mod to_from_sql_impl;

use crate::models::{BookingModel, BookingStatus};

#[async_trait]
pub trait Repository: GenericClient {
    async fn list_bookings_by_user(&self, user_id: Uuid) -> Result<Vec<BookingModel>, Error> {
        let rows = self
            .query(include_str!("sql/list_bookings_by_user.sql"), &[&user_id])
            .await?;
        Ok(rows.iter().map(BookingModel::from_row).collect())
    }

    async fn get_booking_by_id(&self, booking_id: Uuid) -> Result<Option<BookingModel>, Error> {
        let row = self
            .query_opt(include_str!("sql/get_booking_by_id.sql"), &[&booking_id])
            .await?;
        Ok(row.as_ref().map(BookingModel::from_row))
    }

    async fn create_booking(
        &self,
        id: Uuid,
        user_id: Uuid,
        flight_id: Uuid,
        passenger_name: &str,
        passenger_email: &str,
        seat_count: i32,
        price_currency: &str,
        total_price_minor: i64,
    ) -> Result<BookingModel, Error> {
        let row = self
            .query_one(
                include_str!("sql/create_booking.sql"),
                &[
                    &id,
                    &user_id,
                    &flight_id,
                    &passenger_name,
                    &passenger_email,
                    &seat_count,
                    &price_currency,
                    &total_price_minor,
                ],
            )
            .await?;
        Ok(BookingModel::from_row(&row))
    }

    async fn update_booking_status(
        &self,
        booking_id: Uuid,
        status: BookingStatus,
    ) -> Result<Option<BookingModel>, Error> {
        let row = self
            .query_opt(
                include_str!("sql/update_booking_status.sql"),
                &[&booking_id, &status],
            )
            .await?;
        Ok(row.as_ref().map(BookingModel::from_row))
    }
}

impl Repository for Object {}
impl Repository for Transaction<'_> {}
