use crate::api::*;
use postgres_types::Type;
use std::error::Error;
use tokio_postgres::types::{FromSql, ToSql};

macro_rules! impl_to_from_sql {
    ($( $type:ty => $name:expr ), * $(,)?) => {
       $(
impl ToSql for $type where $type: ToString {
    fn to_sql(&self, ty: &Type, out: &mut tokio_postgres::types::private::BytesMut) -> std::result::Result<postgres_types::IsNull, Box<dyn Error + Sync + Send>>
        where
            Self: Sized {
        let s = self.to_string();

        s.to_sql(ty, out)
    }

    fn accepts(ty: &postgres_types::Type) -> bool {
        ty.name() == $name
    }

    postgres_types::to_sql_checked!();
}

impl<'a> FromSql<'a> for $type where $type: core::str::FromStr {
    fn from_sql(ty: &Type, raw: &'a [u8]) -> Result<Self, Box<dyn Error + Sync + Send>> {
        let s = String::from_sql(ty, raw)?;
        Ok(s.parse()?)
    }

    fn accepts(ty: &postgres_types::Type) -> bool {
        ty.name() == $name
    }
}
       )*
    };
}

impl_to_from_sql!(
    ProductStatus => "product_status",
    OrderStatus => "order_status",
    crate::models::UserOperationType => "operation_type",
    UserRole => "user_role",
    DiscountType => "discount_type"
);
