use crate::models::BookingStatus;
use postgres_types::Type;
use std::error::Error;
use tokio_postgres::types::{FromSql, ToSql};

impl ToSql for BookingStatus {
    fn to_sql(
        &self,
        ty: &Type,
        out: &mut tokio_postgres::types::private::BytesMut,
    ) -> std::result::Result<postgres_types::IsNull, Box<dyn Error + Sync + Send>>
    where
        Self: Sized,
    {
        let s = self.as_str();
        s.to_sql(ty, out)
    }

    fn accepts(ty: &postgres_types::Type) -> bool {
        ty.name() == "booking_status"
    }

    postgres_types::to_sql_checked!();
}

impl<'a> FromSql<'a> for BookingStatus {
    fn from_sql(ty: &Type, raw: &'a [u8]) -> Result<Self, Box<dyn Error + Sync + Send>> {
        let s = String::from_sql(ty, raw)?;
        match s.as_str() {
            "CONFIRMED" => Ok(BookingStatus::Confirmed),
            "CANCELLED" => Ok(BookingStatus::Cancelled),
            other => Err(format!("unknown booking_status value: {other}").into()),
        }
    }

    fn accepts(ty: &postgres_types::Type) -> bool {
        ty.name() == "booking_status"
    }
}
