use crate::api::*;
use crate::auth::AuthContext;
use crate::db::Repository;
use crate::state::State;
use deadpool_postgres::Transaction;

pub async fn handle(
    auth: &AuthContext,
    state: &State,
    request: PostOrdersByIdCancelRequest,
) -> anyhow::Result<PostOrdersByIdCancelResponse> {
    match auth.role {
        UserRole::Seller => {
            return Ok(PostOrdersByIdCancelResponse::Forbidden(ErrorResponse {
                error_code: ErrorResponseErrorCode::AccessDenied,
                message: "Sellers cannot cancel orders".to_string(),
                details: None,
            }));
        }
        UserRole::User | UserRole::Admin => {}
    }

    let path = request.path;
    let mut client = state.db.get().await?;

    let tx = client.transaction().await?;

    let result = cancel_order_internal(&tx, auth.user_id, &auth.role, path.id).await;

    match result {
        Ok(response) => {
            tx.commit().await?;
            Ok(PostOrdersByIdCancelResponse::Ok(response))
        }
        Err(err_response) => {
            tx.rollback().await?;
            Ok(err_response)
        }
    }
}

async fn cancel_order_internal(
    tx: &Transaction<'_>,
    user_id: i64,
    user_role: &UserRole,
    order_id: i64,
) -> Result<OrderResponse, PostOrdersByIdCancelResponse> {

    let order = tx
        .get_order(order_id)
        .await
        .map_err(|e| internal_error(e.to_string()))?;

    let order = match order {
        Some(o) => o,
        None => {
            return Err(PostOrdersByIdCancelResponse::NotFound(ErrorResponse {
                error_code: ErrorResponseErrorCode::OrderNotFound,
                message: format!("Order {} not found", order_id),
                details: None,
            }));
        }
    };

    if order.user_id != user_id && !matches!(user_role, UserRole::Admin) {
        return Err(PostOrdersByIdCancelResponse::Forbidden(ErrorResponse {
            error_code: ErrorResponseErrorCode::OrderOwnershipViolation,
            message: "Order belongs to another user".to_string(),
            details: None,
        }));
    }

    if order.status != OrderStatus::Created && order.status != OrderStatus::PaymentPending {
        return Err(PostOrdersByIdCancelResponse::Conflict(ErrorResponse {
            error_code: ErrorResponseErrorCode::InvalidStateTransition,
            message: "Order can only be canceled in CREATED or PAYMENT_PENDING state".to_string(),
            details: None,
        }));
    }

    let items = tx
        .get_order_items(order_id)
        .await
        .map_err(|e| internal_error(e.to_string()))?;

    for item in &items {
        tx.restore_stock(item.item.product_id, item.item.quantity as i32)
            .await
            .map_err(|e| internal_error(e.to_string()))?;
    }

    if let Some(promo_id) = order.promo_code_id {
        tx.decrement_promo_uses(promo_id)
            .await
            .map_err(|e| internal_error(e.to_string()))?;
    }

    let mut updated_order = tx
        .update_order_status(order_id, OrderStatus::Canceled)
        .await
        .map_err(|e| internal_error(e.to_string()))?;

    let order_items = items
        .into_iter()
        .map(|item| OrderItemResponse {
            id: item.id,
            item: item.item,
            price_at_order: item.price_at_order,
        })
        .collect();

    updated_order.items = order_items;
    updated_order.promo_code = order.promo_code;

    Ok(updated_order)
}

fn internal_error(msg: String) -> PostOrdersByIdCancelResponse {
    PostOrdersByIdCancelResponse::BadRequest(ErrorResponse {
        error_code: ErrorResponseErrorCode::ValidationError,
        message: msg,
        details: None,
    })
}
