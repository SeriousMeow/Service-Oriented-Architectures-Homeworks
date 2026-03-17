use crate::api::*;
use crate::auth::AuthContext;
use crate::db::Repository;
use crate::handlers::orders::common;
use crate::models::UserOperationType;
use crate::state::State;
use deadpool_postgres::Transaction;
use rust_decimal::Decimal;
use std::collections::HashMap;
use serde_json::json;

pub async fn handle(
    auth: &AuthContext,
    state: &State,
    request: PostOrdersRequest,
) -> anyhow::Result<PostOrdersResponse> {
    match auth.role {
        UserRole::Seller => {
            return Ok(PostOrdersResponse::Forbidden(ErrorResponse {
                error_code: ErrorResponseErrorCode::AccessDenied,
                message: "Sellers cannot create orders".to_string(),
                details: None,
            }));
        }
        UserRole::User | UserRole::Admin => {}
    }

    let body = request.body;
    let mut client = state.db.get().await?;

    let tx = client.transaction().await?;

    let response = create_order_internal(&tx, state, auth.user_id, body).await?;

    tx.commit().await?;

    Ok(response)
}

async fn create_order_internal(
    tx: &Transaction<'_>,
    state: &State,
    user_id: i64,
    body: OrderCreateRequest,
) -> anyhow::Result<PostOrdersResponse> {

    if let Some(last_op_time) = tx
        .get_last_user_operation(user_id, UserOperationType::CreateOrder)
        .await?
    {
        let elapsed = chrono::Utc::now() - last_op_time;
        if elapsed.num_minutes() < state.config.order_rate_limit_minutes {
            return Ok(PostOrdersResponse::TooManyRequests(ErrorResponse {
                error_code: ErrorResponseErrorCode::OrderLimitExceeded,
                message: format!(
                    "Order creation rate limit exceeded. Please wait {} minutes.",
                    state.config.order_rate_limit_minutes - elapsed.num_minutes()
                ),
                details: None,
            }));
        }
    }

    if tx.check_active_order(user_id).await? {
        return Ok(PostOrdersResponse::Conflict(ErrorResponse {
            error_code: ErrorResponseErrorCode::OrderHasActive,
            message: "User already has an active order".to_string(),
            details: None,
        }));
    }

    let mut products_map = HashMap::new();
    let mut insufficient_stock = Vec::new();

    for item in &body.items {
        let product = tx.try_get_product(item.product_id).await?;

        let product = match product {
            Some(p) => p,
            None => {
                return Ok(PostOrdersResponse::NotFound(ErrorResponse {
                    error_code: ErrorResponseErrorCode::ProductNotFound,
                    message: format!("Product {} not found", item.product_id),
                    details: None,
                }));
            }
        };

        if product.status != ProductStatus::Active {
            return Ok(PostOrdersResponse::Conflict(ErrorResponse {
                error_code: ErrorResponseErrorCode::ProductInactive,
                message: format!("Product {} is not active", item.product_id),
                details: None,
            }));
        }

        if product.stock < item.quantity {
            insufficient_stock.push(json!({
                "product_id": item.product_id,
                "requested": item.quantity,
                "available": product.stock
            }));
        }

        products_map.insert(item.product_id, product);
    }

    if !insufficient_stock.is_empty() {
        let mut details_map = HashMap::new();
        details_map.insert(
            "products".to_string(),
            json!(insufficient_stock),
        );
        return Ok(PostOrdersResponse::Conflict(ErrorResponse {
            error_code: ErrorResponseErrorCode::InsufficientStock,
            message: "Insufficient stock for one or more products".to_string(),
            details: Some(details_map),
        }));
    }

    for item in &body.items {
        tx.reserve_stock(item.product_id, item.quantity as i32)
            .await?;
    }

    let mut total_amount = Decimal::ZERO;
    for item in &body.items {
        let product = products_map.get(&item.product_id).unwrap();
        let price = product.price;
        total_amount += price * Decimal::from(item.quantity);
    }

    let mut discount_amount = Decimal::ZERO;
    let mut promo_code_id = None;
    let promo_code_str = body.promo_code.clone();

    if let Some(ref code) = body.promo_code {
        let promo = tx.get_promo_code(code).await?;

        let promo = match promo {
            Some(p) => p,
            None => {
                return Ok(PostOrdersResponse::UnprocessableEntity(ErrorResponse {
                    error_code: ErrorResponseErrorCode::PromoCodeInvalid,
                    message: "Promo code not found".to_string(),
                    details: None,
                }));
            }
        };

        let now = chrono::Utc::now();
        if !promo.active
            || promo.current_uses >= promo.max_uses
            || now < promo.valid_from
            || now > promo.valid_until
        {
            return Ok(PostOrdersResponse::UnprocessableEntity(ErrorResponse {
                error_code: ErrorResponseErrorCode::PromoCodeInvalid,
                message: "Promo code is invalid, expired, or exhausted".to_string(),
                details: None,
            }));
        }

        if total_amount < promo.min_order_amount {
            return Ok(PostOrdersResponse::UnprocessableEntity(ErrorResponse {
                error_code: ErrorResponseErrorCode::PromoCodeMinAmount,
                message: format!(
                    "Order total {} is below minimum {} for promo code",
                    total_amount, promo.min_order_amount
                ),
                details: None,
            }));
        }

        discount_amount = common::calculate_discount(&promo, total_amount);

        total_amount -= discount_amount;
        promo_code_id = Some(promo.id);

        tx.increment_promo_uses(promo.id).await?;
    }

    let order = tx
        .create_order(
            user_id,
            OrderStatus::Created,
            promo_code_id,
            total_amount,
            discount_amount,
        )
        .await?;

    let mut order_items = Vec::new();
    for item in &body.items {
        let product = products_map.get(&item.product_id).unwrap();
        let price = product.price;

        let order_item = tx
            .create_order_item(order.id, item.product_id, item.quantity as i32, price)
            .await?;

        order_items.push(order_item);
    }

    tx.record_user_operation(user_id, UserOperationType::CreateOrder).await?;

    Ok(PostOrdersResponse::Created(OrderResponse {
        items: order_items,
        promo_code: promo_code_str,
        ..order
    }))
}
