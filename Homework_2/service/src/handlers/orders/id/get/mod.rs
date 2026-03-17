use crate::api::*;
use crate::auth::AuthContext;
use crate::db::Repository;
use crate::state::State;

pub async fn handle(
    auth: &AuthContext,
    state: &State,
    request: GetOrdersByIdRequest,
) -> anyhow::Result<GetOrdersByIdResponse> {
    match auth.role {
        UserRole::Seller => {
            return Ok(GetOrdersByIdResponse::Forbidden(ErrorResponse {
                error_code: ErrorResponseErrorCode::AccessDenied,
                message: "Sellers cannot view orders".to_string(),
                details: None,
            }));
        }
        UserRole::User | UserRole::Admin => {}
    }

    let path = request.path;
    let client = state.db.get().await?;

    let order = match client.get_order(path.id).await? {
        Some(o) => o,
        None => {
            return Ok(GetOrdersByIdResponse::NotFound(ErrorResponse {
                error_code: ErrorResponseErrorCode::OrderNotFound,
                message: format!("Order {} not found", path.id),
                details: None,
            }));
        }
    };

    if matches!(auth.role, UserRole::User) && order.user_id != auth.user_id {
        return Ok(GetOrdersByIdResponse::Forbidden(ErrorResponse {
            error_code: ErrorResponseErrorCode::OrderOwnershipViolation,
            message: "You can only view your own orders".to_string(),
            details: None,
        }));
    }

    let items = client.get_order_items(path.id).await?;

    let order_response = OrderResponse {
        id: order.id,
        user_id: order.user_id,
        status: order.status,
        promo_code: order.promo_code,
        total_amount: order.total_amount.to_string(),
        discount_amount: order.discount_amount.to_string(),
        items,
        created_at: order.created_at,
        updated_at: order.updated_at,
    };

    Ok(GetOrdersByIdResponse::Ok(order_response))
}
