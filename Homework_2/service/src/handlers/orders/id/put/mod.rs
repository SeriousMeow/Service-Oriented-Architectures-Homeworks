use crate::api::*;
use crate::auth::AuthContext;
use crate::db::Repository;
use crate::handlers::orders::common;
use crate::models::UserOperationType;
use crate::state::State;
use deadpool_postgres::Transaction;
use rust_decimal::Decimal;
use std::collections::HashMap;

pub async fn handle(
    auth: &AuthContext,
    state: &State,
    request: PutOrdersByIdRequest,
) -> anyhow::Result<PutOrdersByIdResponse> {
    match auth.role {
        UserRole::Seller => {
            return Ok(PutOrdersByIdResponse::Forbidden(ErrorResponse {
                error_code: ErrorResponseErrorCode::AccessDenied,
                message: "Sellers cannot update orders".to_string(),
                details: None,
            }));
        }
        UserRole::User | UserRole::Admin => {}
    }

    let path = request.path;
    let body = request.body;
    let mut client = state.db.get().await?;

    let tx = client.transaction().await?;

    let response = update_order_internal(&tx, state, auth.user_id, &auth.role, path.id, body).await?;

    tx.commit().await?;

    Ok(response)
}

async fn update_order_internal(
    tx: &Transaction<'_>,
    state: &State,
    user_id: i64,
    user_role: &UserRole,
    order_id: i64,
    body: OrderUpdateRequest,
) -> anyhow::Result<PutOrdersByIdResponse> {

    let order = tx.get_order(order_id).await?;

    let order = match order {
        Some(o) => o,
        None => {
            return Ok(PutOrdersByIdResponse::NotFound(ErrorResponse {
                error_code: ErrorResponseErrorCode::OrderNotFound,
                message: format!("Order {} not found", order_id),
                details: None,
            }));
        }
    };

    if order.user_id != user_id && !matches!(user_role, UserRole::Admin) {
        return Ok(PutOrdersByIdResponse::Forbidden(ErrorResponse {
            error_code: ErrorResponseErrorCode::OrderOwnershipViolation,
            message: "Order belongs to another user".to_string(),
            details: None,
        }));
    }

    if order.status != OrderStatus::Created {
        return Ok(PutOrdersByIdResponse::Conflict(ErrorResponse {
            error_code: ErrorResponseErrorCode::InvalidStateTransition,
            message: "Order can only be updated in CREATED state".to_string(),
            details: None,
        }));
    }

    if let Some(last_op_time) = tx
        .get_last_user_operation(user_id, UserOperationType::UpdateOrder)
        .await?
    {
        let elapsed = chrono::Utc::now() - last_op_time;
        if elapsed.num_minutes() < state.config.order_rate_limit_minutes {
            return Ok(PutOrdersByIdResponse::TooManyRequests(ErrorResponse {
                error_code: ErrorResponseErrorCode::OrderLimitExceeded,
                message: format!(
                    "Order update rate limit exceeded. Please wait {} minutes.",
                    state.config.order_rate_limit_minutes - elapsed.num_minutes()
                ),
                details: None,
            }));
        }
    }

    let old_items = tx.get_order_items(order_id).await?;

    for old_item in &old_items {
        tx.restore_stock(old_item.item.product_id, old_item.item.quantity as i32)
            .await?;
    }

    let mut products_map = HashMap::new();
    for item in &body.items {
        let product = tx.try_get_product(item.product_id).await?;

        let product = match product {
            Some(p) => p,
            None => {
                return Ok(PutOrdersByIdResponse::NotFound(ErrorResponse {
                    error_code: ErrorResponseErrorCode::ProductNotFound,
                    message: format!("Product {} not found", item.product_id),
                    details: None,
                }));
            }
        };

        if product.status != ProductStatus::Active {
            return Ok(PutOrdersByIdResponse::Conflict(ErrorResponse {
                error_code: ErrorResponseErrorCode::ProductInactive,
                message: format!("Product {} is not active", item.product_id),
                details: None,
            }));
        }

        products_map.insert(item.product_id, product);
    }

    let mut insufficient_stock = Vec::new();
    for item in &body.items {
        let product = products_map.get(&item.product_id).unwrap();
        if product.stock < item.quantity {
            let mut details = HashMap::new();
            details.insert("product_id".to_string(), serde_json::json!(item.product_id));
            details.insert("requested".to_string(), serde_json::json!(item.quantity));
            details.insert("available".to_string(), serde_json::json!(product.stock));
            insufficient_stock.push(details);
        }
    }

    if !insufficient_stock.is_empty() {
        let mut details_map = HashMap::new();
        details_map.insert(
            "products".to_string(),
            serde_json::json!(insufficient_stock),
        );
        return Ok(PutOrdersByIdResponse::Conflict(ErrorResponse {
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
    let mut new_promo_code_id = order.promo_code_id;
    let mut promo_code_str = order.promo_code.clone();

    if let Some(promo_id) = order.promo_code_id {
        let promo = tx
            .get_promo_code(&order.promo_code.clone().unwrap_or_default())
            .await?;

        if let Some(promo) = promo {
            if total_amount >= promo.min_order_amount {
                discount_amount = common::calculate_discount(&promo, total_amount);
            } else {
                tx.decrement_promo_uses(promo_id).await?;
                new_promo_code_id = None;
                promo_code_str = None;
            }
        }
    }

    total_amount -= discount_amount;

    tx.delete_order_items(order_id).await?;

    let mut order_items = Vec::new();
    for item in &body.items {
        let product = products_map.get(&item.product_id).unwrap();
        let price = product.price;

        let order_item = tx
            .create_order_item(order_id, item.product_id, item.quantity as i32, price)
            .await?;

        order_items.push(order_item);
    }

    let mut updated_order = tx
        .update_order(order_id, total_amount, discount_amount, new_promo_code_id)
        .await?;

    tx.record_user_operation(user_id, UserOperationType::UpdateOrder).await?;

    updated_order.items = order_items;
    updated_order.promo_code = promo_code_str;

    Ok(PutOrdersByIdResponse::Ok(updated_order))
}

fn internal_error(msg: String) -> PutOrdersByIdResponse {
    PutOrdersByIdResponse::BadRequest(ErrorResponse {
        error_code: ErrorResponseErrorCode::ValidationError,
        message: msg,
        details: None,
    })
}
