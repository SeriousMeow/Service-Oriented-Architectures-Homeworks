use crate::api::*;
use crate::auth::AuthContext;
use crate::db::Repository;
use crate::state::State;
use rust_decimal::Decimal;
use std::str::FromStr;

pub async fn handle(
    auth: &AuthContext,
    state: &State,
    request: PutProductsByIdRequest,
) -> anyhow::Result<PutProductsByIdResponse> {
    match auth.role {
        UserRole::User => {
            return Ok(PutProductsByIdResponse::Forbidden(ErrorResponse {
                error_code: ErrorResponseErrorCode::AccessDenied,
                message: "Users cannot update products".to_string(),
                details: None,
            }));
        }
        UserRole::Seller | UserRole::Admin => {}
    }

    let path = request.path;
    let body = request.body;
    let price = body.price
        .map(|p| Decimal::from_str(&p).map_err(|_| anyhow::anyhow!("Invalid price format")))
        .transpose()?;

    let client = state.db.get().await?;

    if matches!(auth.role, UserRole::Seller) {
        let product = client.try_get_product(path.id).await?;
        match product {
            None => {
                return Ok(PutProductsByIdResponse::NotFound(ErrorResponse {
                    details: None,
                    error_code: ErrorResponseErrorCode::ProductNotFound,
                    message: "Product not found".to_string(),
                }));
            }
            Some(p) if p.seller_id != Some(auth.user_id) => {
                return Ok(PutProductsByIdResponse::Forbidden(ErrorResponse {
                    error_code: ErrorResponseErrorCode::AccessDenied,
                    message: "Sellers can only update their own products".to_string(),
                    details: None,
                }));
            }
            _ => {}
        }
    }

    if client
        .update_product(
            path.id,
            body.name,
            body.description,
            price,
            body.stock,
            body.category,
            body.status,
        )
        .await?
    {
        Ok(PutProductsByIdResponse::NoContent)
    } else {
        Ok(PutProductsByIdResponse::NotFound(ErrorResponse {
            details: None,
            error_code: ErrorResponseErrorCode::ProductNotFound,
            message: "Product not found".to_string(),
        }))
    }
}
