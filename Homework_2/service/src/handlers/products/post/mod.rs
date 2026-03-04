use crate::api::*;
use crate::auth::AuthContext;
use crate::db::Repository;
use crate::state::State;
use rust_decimal::Decimal;
use std::str::FromStr;

pub async fn handle(
    auth: &AuthContext,
    state: &State,
    request: PostProductsRequest,
) -> anyhow::Result<PostProductsResponse> {
    match auth.role {
        UserRole::User => {
            return Ok(PostProductsResponse::BadRequest(ErrorResponse {
                error_code: ErrorResponseErrorCode::AccessDenied,
                message: "Users cannot create products".to_string(),
                details: None,
            }));
        }
        UserRole::Seller | UserRole::Admin => {}
    }

    let body = request.body;
    let client = state.db.get().await?;

    let seller_id = match auth.role {
        UserRole::Seller => Some(auth.user_id),
        UserRole::Admin => None,
        UserRole::User => unreachable!(),
    };

    let price = Decimal::from_str(&body.price)
        .map_err(|_| anyhow::anyhow!("Invalid price format"))?;

    client
        .create_product(
            body.name,
            body.description,
            price,
            body.stock,
            body.category,
            body.status,
            seller_id,
        )
        .await?;
    Ok(PostProductsResponse::NoContent)
}
