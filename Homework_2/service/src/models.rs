use crate::api;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use tokio_postgres::row::Row;
use anyhow::Result;
use strum_macros::{Display, EnumString};

#[derive(EnumString, Display, Debug)]
pub enum UserOperationType {
    #[strum(serialize = "CREATE_ORDER")]
    CreateOrder,
    #[strum(serialize = "UPDATE_ORDER")]
    UpdateOrder,
}

#[derive(Debug, Clone)]
pub struct Product {
    pub id: api::ProductId,
    pub name: String,
    pub description: Option<String>,
    pub price: Decimal,
    pub stock: api::ProductStock,
    pub category: String,
    pub status: api::ProductStatus,
    pub seller_id: Option<api::UserId>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl TryFrom<Row> for Product {
    type Error = anyhow::Error;

    fn try_from(row: Row) -> Result<Self, Self::Error> {
        let price: Decimal = row.try_get(3)?;
        Ok(Self {
            id: row.try_get(0)?,
            name: row.try_get(1)?,
            description: row.try_get(2)?,
            price,
            stock: row.try_get::<_, i32>(4)? as i64,
            category: row.try_get(5)?,
            status: row.try_get(6)?,
            created_at: row.try_get(7)?,
            updated_at: row.try_get(8)?,
            seller_id: row.try_get(9).ok(),
        })
    }
}

impl From<Product> for api::Product {
    fn from(db: Product) -> Self {
        api::Product {
            id: db.id,
            name: db.name,
            description: db.description,
            price: db.price.to_string(),
            stock: db.stock,
            category: db.category,
            status: db.status,
            created_at: db.created_at,
            updated_at: db.updated_at,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Order {
    pub id: api::OrderId,
    pub user_id: api::UserId,
    pub status: api::OrderStatus,
    pub promo_code: Option<String>,
    pub promo_code_id: Option<i64>,
    pub total_amount: Decimal,
    pub discount_amount: Decimal,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl TryFrom<Row> for Order {
    type Error = anyhow::Error;

    fn try_from(row: Row) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.try_get(0)?,
            user_id: row.try_get(1)?,
            status: row.try_get(2)?,
            promo_code_id: row.try_get(3)?,
            total_amount: row.try_get(4)?,
            discount_amount: row.try_get(5)?,
            created_at: row.try_get(6)?,
            updated_at: row.try_get(7)?,
            promo_code: row.try_get(8)?,
        })
    }
}

impl From<Order> for api::OrderResponse {
    fn from(o: Order) -> Self {
        api::OrderResponse {
            id: o.id,
            user_id: o.user_id,
            status: o.status,
            promo_code: o.promo_code,
            total_amount: o.total_amount.to_string(),
            discount_amount: o.discount_amount.to_string(),
            items: Vec::new(),
            created_at: o.created_at,
            updated_at: o.updated_at,
        }
    }
}

#[derive(Debug, Clone)]
pub struct OrderItem {
    pub id: api::OrderItemId,
    pub order_id: api::OrderId,
    pub product_id: api::ProductId,
    pub quantity: i32,
    pub price_at_order: Decimal,
}

impl TryFrom<Row> for OrderItem {
    type Error = anyhow::Error;

    fn try_from(row: Row) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.try_get(0)?,
            order_id: row.try_get(1)?,
            product_id: row.try_get(2)?,
            quantity: row.try_get(3)?,
            price_at_order: row.try_get(4)?,
        })
    }
}

impl From<OrderItem> for api::OrderItemResponse {
    fn from(db: OrderItem) -> Self {
        api::OrderItemResponse {
            id: db.id,
            item: api::OrderItem {
                product_id: db.product_id,
                quantity: db.quantity as i64,
            },
            price_at_order: db.price_at_order.to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PromoCode {
    pub id: api::PromoCodeId,
    pub code: String,
    pub discount_type: api::DiscountType,
    pub discount_value: Decimal,
    pub min_order_amount: Decimal,
    pub max_uses: i32,
    pub current_uses: i32,
    pub valid_from: DateTime<Utc>,
    pub valid_until: DateTime<Utc>,
    pub active: bool,
}

impl TryFrom<Row> for PromoCode {
    type Error = anyhow::Error;

    fn try_from(row: Row) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.try_get(0)?,
            code: row.try_get(1)?,
            discount_type: row.try_get(2)?,
            discount_value: row.try_get(3)?,
            min_order_amount: row.try_get(4)?,
            max_uses: row.try_get(5)?,
            current_uses: row.try_get(6)?,
            valid_from: row.try_get(7)?,
            valid_until: row.try_get(8)?,
            active: row.try_get(9)?
        })
    }
}

impl From<PromoCode> for api::PromoCodeObject {
    fn from(db: PromoCode) -> Self {
        api::PromoCodeObject {
            id: Some(db.id),
            code: db.code,
            discount_type: db.discount_type,
            discount_value: db.discount_value.to_string(),
            min_order_amount: db.min_order_amount.to_string(),
            max_uses: db.max_uses as i64,
            current_uses: Some(db.current_uses as i64),
            valid_from: db.valid_from,
            valid_until: db.valid_until,
            active: Some(db.active),
        }
    }
}

#[derive(Debug, Clone)]
pub struct User {
    pub id: api::UserId,
    pub email: String,
    pub password_hash: String,
    pub role: api::UserRole,
    pub created_at: DateTime<Utc>,
}

impl TryFrom<Row> for User {
    type Error = anyhow::Error;

    fn try_from(row: Row) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.try_get(0)?,
            email: row.try_get(1)?,
            password_hash: row.try_get(2)?,
            role: row.try_get(3)?,
            created_at: row.try_get(4)?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct RefreshToken {
    pub id: i64,
    pub user_id: api::UserId,
    pub token_hash: String,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

impl TryFrom<Row> for RefreshToken {
    type Error = anyhow::Error;

    fn try_from(row: Row) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.try_get(0)?,
            user_id: row.try_get(1)?,
            token_hash: row.try_get(2)?,
            expires_at: row.try_get(3)?,
            created_at: row.try_get(4)?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct UserOperation {
    pub id: i64,
    pub user_id: api::UserId,
    pub operation_type: String,
    pub created_at: DateTime<Utc>,
}
