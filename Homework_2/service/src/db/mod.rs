use crate::api;
use crate::models;
use anyhow::Result;
use deadpool_postgres::GenericClient;
use deadpool_postgres::{Object, Transaction};
use rust_decimal::Decimal;
use map_ok::MapOk;

mod to_from_sql_impl;

pub trait Repository: GenericClient {
    async fn try_get_product(&self, id: api::ProductId) -> Result<Option<models::Product>> {
        let statement = include_str!("sql/get_product.sql");

        Ok(self.query_opt(statement, &[&id]).await?.map(|row| row.try_into()).transpose()?)
    }

    async fn get_products(
        &self,
        status: Option<api::ProductStatus>,
        category: Option<api::ProductCategory>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<models::Product>> {
        let statement = include_str!("sql/get_products.sql");

        Ok(self
            .query(statement, &[&status, &category, &limit, &offset])
            .await?
            .into_iter()
            .map(|row| row.try_into())
            .collect::<Result<Vec<models::Product>>>()?)
    }

    async fn count_products(
        &self,
        status: Option<api::ProductStatus>,
        category: Option<api::ProductCategory>,
    ) -> Result<i64> {
        let statement = include_str!("sql/count_products.sql");

        Ok(self
            .query_one(statement, &[&status, &category])
            .await?
            .try_get(0)?)
    }

    async fn create_product(
        &self,
        name: String,
        description: Option<String>,
        price: Decimal,
        stock: api::ProductStock,
        category: api::ProductCategory,
        status: api::ProductStatus,
        seller_id: Option<i64>,
    ) -> Result<()> {
        let statement = include_str!("sql/create_product.sql");

        self.execute(
            statement,
            &[&name, &description, &price, &(stock as i32), &category, &status, &seller_id],
        )
        .await?;
        Ok(())
    }

    async fn update_product(
        &self,
        id: api::ProductId,
        name: Option<String>,
        description: Option<String>,
        price: Option<Decimal>,
        stock: Option<api::ProductStock>,
        category: Option<String>,
        status: Option<api::ProductStatus>,
    ) -> Result<bool> {
        let statement = include_str!("sql/update_product.sql");
        let stock_i32 = stock.map(|s| s as i32);
        let rows = self
            .execute(
                statement,
                &[&id, &name, &description, &price, &stock_i32, &category, &status],
            )
            .await?;
        Ok(rows > 0)
    }

    async fn archive_product(&self, id: api::ProductId) -> Result<bool> {
        let statement = include_str!("sql/archive_product.sql");
        let rows = self.execute(statement, &[&id]).await?;
        Ok(rows > 0)
    }

    async fn get_last_user_operation(
        &self,
        user_id: i64,
        operation_type: models::UserOperationType,
    ) -> Result<Option<chrono::DateTime<chrono::Utc>>> {
        let statement = include_str!("sql/get_last_user_operation.sql");
        Ok(self
            .query_opt(statement, &[&user_id, &operation_type])
            .await?
            .map(|row| row.try_get(3))
            .transpose()?)
    }

    async fn check_active_order(&self, user_id: i64) -> Result<bool> {
        let statement = include_str!("sql/check_active_order.sql");
        Ok(self.query_opt(statement, &[&user_id]).await?.is_some())
    }

    async fn get_promo_code(&self, code: &str) -> Result<Option<models::PromoCode>> {
        let statement = include_str!("sql/get_promo_code.sql");
        Ok(self.query_opt(statement, &[&code]).await?.map(|row| row.try_into()).transpose()?)
    }

    async fn create_promo_code(
        &self,
        code: String,
        discount_type: api::DiscountType,
        discount_value: Decimal,
        min_order_amount: Decimal,
        max_uses: i32,
        current_uses: i32,
        valid_from: chrono::DateTime<chrono::Utc>,
        valid_until: chrono::DateTime<chrono::Utc>,
        active: bool,
    ) -> Result<models::PromoCode> {
        let statement = include_str!("sql/create_promo_code.sql");
        Ok(self.query_one(statement, &[&code, &discount_type, &discount_value, &min_order_amount, &max_uses, &current_uses, &valid_from, &valid_until, &active]).await?.try_into()?)
    }

    async fn increment_promo_uses(&self, promo_id: i64) -> Result<()> {
        let statement = include_str!("sql/increment_promo_uses.sql");
        self.execute(statement, &[&promo_id]).await?;
        Ok(())
    }

    async fn decrement_promo_uses(&self, promo_id: i64) -> Result<()> {
        let statement = include_str!("sql/decrement_promo_uses.sql");
        self.execute(statement, &[&promo_id]).await?;
        Ok(())
    }

    async fn reserve_stock(&self, product_id: api::ProductId, quantity: i32) -> Result<()> {
        let statement = include_str!("sql/reserve_stock.sql");
        self.execute(statement, &[&product_id, &quantity]).await?;
        Ok(())
    }

    async fn restore_stock(&self, product_id: api::ProductId, quantity: i32) -> Result<()> {
        let statement = include_str!("sql/restore_stock.sql");
        self.execute(statement, &[&product_id, &quantity]).await?;
        Ok(())
    }

    async fn create_order(
        &self,
        user_id: i64,
        status: api::OrderStatus,
        promo_code_id: Option<i64>,
        total_amount: Decimal,
        discount_amount: Decimal,
    ) -> Result<api::OrderResponse> {
        let statement = include_str!("sql/create_order.sql");
        Ok(models::Order::try_from(self.query_one(statement, &[&user_id, &status, &promo_code_id, &total_amount, &discount_amount]).await?).map(Into::into)?)
    }

    async fn create_order_item(
        &self,
        order_id: i64,
        product_id: i64,
        quantity: i32,
        price_at_order: Decimal,
    ) -> Result<api::OrderItemResponse> {
        let statement = include_str!("sql/create_order_item.sql");
        Ok(models::OrderItem::try_from(self.query_one(statement, &[&order_id, &product_id, &quantity, &price_at_order]).await?).map(Into::into)?)
    }

    async fn get_order(&self, order_id: i64) -> Result<Option<models::Order>> {
        let statement = include_str!("sql/get_order.sql");
        Ok(self.query_opt(statement, &[&order_id]).await?.map(|row| row.try_into()).transpose()?)
    }

    async fn get_order_items(&self, order_id: i64) -> Result<Vec<api::OrderItemResponse>> {
        let statement = include_str!("sql/get_order_items.sql");
        
        self
            .query(statement, &[&order_id])
            .await?
            .into_iter()
            .map(|row| row.try_into())
            .map_ok(|item: models::OrderItem| item.into())
            .collect::<Result<Vec<api::OrderItemResponse>>>()
    }

    async fn delete_order_items(&self, order_id: i64) -> Result<()> {
        let statement = include_str!("sql/delete_order_items.sql");
        self.execute(statement, &[&order_id]).await?;
        Ok(())
    }

    async fn update_order(
        &self,
        order_id: i64,
        total_amount: Decimal,
        discount_amount: Decimal,
        promo_code_id: Option<i64>,
    ) -> Result<api::OrderResponse> {
        let statement = include_str!("sql/update_order.sql");
        Ok(models::Order::try_from(self.query_one(statement, &[&order_id, &total_amount, &discount_amount, &promo_code_id]).await?).map(Into::into)?)
    }

    async fn update_order_status(
        &self,
        order_id: i64,
        status: api::OrderStatus,
    ) -> Result<api::OrderResponse> {
        let statement = include_str!("sql/update_order_status.sql");
        Ok(models::Order::try_from(self.query_one(statement, &[&order_id, &status]).await?).map(Into::into)?)
    }

    async fn record_user_operation(&self, user_id: i64, operation_type: models::UserOperationType) -> Result<()> {
        let statement = include_str!("sql/record_user_operation.sql");
        self.execute(statement, &[&user_id, &operation_type])
            .await?;
        Ok(())
    }

    async fn create_user(
        &self,
        email: &str,
        password_hash: &str,
        role: api::UserRole,
    ) -> Result<models::User> {
        let statement = include_str!("sql/create_user.sql");
        Ok(self.query_one(statement, &[&email, &password_hash, &role]).await?.try_into()?)
    }

    async fn get_user_by_email(&self, email: &str) -> Result<Option<models::User>> {
        let statement = include_str!("sql/get_user_by_email.sql");
        Ok(self.query_opt(statement, &[&email]).await?.map(|row| row.try_into()).transpose()?)
    }

    async fn get_user_by_id(&self, user_id: i64) -> Result<Option<models::User>> {
        let statement = include_str!("sql/get_user_by_id.sql");
        Ok(self.query_opt(statement, &[&user_id]).await?.map(|row| row.try_into()).transpose()?)
    }

    async fn create_refresh_token(
        &self,
        user_id: i64,
        token: &str,
        expires_at: chrono::DateTime<chrono::Utc>,
    ) -> Result<models::RefreshToken> {
        let statement = include_str!("sql/create_refresh_token.sql");
        Ok(self.query_one(statement, &[&user_id, &token, &expires_at]).await?.try_into()?)
    }

    async fn get_refresh_token(&self, token: &str) -> Result<Option<models::RefreshToken>> {
        let statement = include_str!("sql/get_refresh_token.sql");
        Ok(self.query_opt(statement, &[&token]).await?.map(|row| row.try_into()).transpose()?)
    }

    async fn delete_refresh_token(&self, token: &str) -> Result<()> {
        let statement = include_str!("sql/delete_refresh_token.sql");
        self.execute(statement, &[&token]).await?;
        Ok(())
    }

    async fn delete_refresh_tokens_by_user(&self, user_id: i64) -> Result<()> {
        let statement = include_str!("sql/delete_refresh_tokens_by_user.sql");
        self.execute(statement, &[&user_id]).await?;
        Ok(())
    }
}

impl Repository for Object {}
impl<'a> Repository for Transaction<'a> {}
