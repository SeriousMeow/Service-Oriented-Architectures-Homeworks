use async_trait::async_trait;
use chrono::NaiveDate;
use deadpool_postgres::{GenericClient, Object, Transaction};
use tokio_postgres::Error;
use uuid::Uuid;

use crate::models::{FlightModel, FlightStatusDb, SeatReservationModel};

#[async_trait]
pub trait Repository: GenericClient {
    async fn search_flights(
        &self,
        origin: &str,
        destination: &str,
        departure_date: Option<NaiveDate>,
    ) -> Result<Vec<FlightModel>, Error> {
        match departure_date {
            Some(day) => {
                let rows = self
                    .query(
                        include_str!("sql/search_flights_by_date.sql"),
                        &[&origin, &destination, &day],
                    )
                    .await?;
                Ok(rows.iter().map(FlightModel::from_row).collect())
            }
            None => {
                let rows = self
                    .query(
                        include_str!("sql/search_flights.sql"),
                        &[&origin, &destination],
                    )
                    .await?;
                Ok(rows.iter().map(FlightModel::from_row).collect())
            }
        }
    }

    async fn get_flight_by_id(&self, flight_id: Uuid) -> Result<Option<FlightModel>, Error> {
        let row = self
            .query_opt(include_str!("sql/get_flight_by_id.sql"), &[&flight_id])
            .await?;
        Ok(row.as_ref().map(FlightModel::from_row))
    }

    async fn get_reservation_by_booking_id(
        &self,
        booking_id: Uuid,
    ) -> Result<Option<SeatReservationModel>, Error> {
        let row = self
            .query_opt(
                include_str!("sql/get_reservation_by_booking_id.sql"),
                &[&booking_id],
            )
            .await?;
        Ok(row.as_ref().map(SeatReservationModel::from_row))
    }

    async fn get_reservation_by_booking_id_for_update(
        &self,
        booking_id: Uuid,
    ) -> Result<Option<SeatReservationModel>, Error> {
        let row = self
            .query_opt(
                include_str!("sql/get_reservation_for_update.sql"),
                &[&booking_id],
            )
            .await?;
        Ok(row.as_ref().map(SeatReservationModel::from_row))
    }

    async fn try_reserve_seats_on_scheduled_flight(
        &self,
        flight_id: Uuid,
        seat_count: i32,
    ) -> Result<Option<FlightModel>, Error> {
        let row = self
            .query_opt(
                include_str!("sql/try_reserve_seats.sql"),
                &[&seat_count, &flight_id],
            )
            .await?;
        Ok(row.as_ref().map(FlightModel::from_row))
    }

    async fn lock_flight_status_and_available_for_update(
        &self,
        flight_id: Uuid,
    ) -> Result<Option<(FlightStatusDb, i32)>, Error> {
        let row = self
            .query_opt(
                include_str!("sql/lock_flight_for_update.sql"),
                &[&flight_id],
            )
            .await?;
        Ok(row.map(|r| (r.get(0), r.get(1))))
    }

    async fn create_active_reservation(
        &self,
        booking_id: Uuid,
        flight_id: Uuid,
        seat_count: i32,
    ) -> Result<SeatReservationModel, Error> {
        let row = self
            .query_one(
                include_str!("sql/create_reservation.sql"),
                &[&booking_id, &flight_id, &seat_count],
            )
            .await?;
        Ok(SeatReservationModel::from_row(&row))
    }

    async fn release_flight_seats(
        &self,
        flight_id: Uuid,
        seat_count: i32,
    ) -> Result<Option<FlightModel>, Error> {
        let row = self
            .query_opt(
                include_str!("sql/release_flight_seats.sql"),
                &[&seat_count, &flight_id],
            )
            .await?;
        Ok(row.as_ref().map(FlightModel::from_row))
    }

    async fn mark_reservation_released(
        &self,
        reservation_id: Uuid,
    ) -> Result<SeatReservationModel, Error> {
        let row = self
            .query_one(
                include_str!("sql/mark_reservation_released.sql"),
                &[&reservation_id],
            )
            .await?;
        Ok(SeatReservationModel::from_row(&row))
    }
}

impl Repository for Object {}
impl Repository for Transaction<'_> {}
