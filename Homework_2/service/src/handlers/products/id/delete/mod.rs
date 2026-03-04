use crate::api::*;
use crate::auth::AuthContext;
use crate::db::Repository;
use crate::state::State;

pub async fn handle(
    auth: &AuthContext,
    state: &State,
    request: DeleteProductsByIdRequest,
) -> anyhow::Result<PutProductsByIdResponse> {
    match auth.role {
        UserRole::User => {
            return Ok(PutProductsByIdResponse::Forbidden(ErrorResponse {
                error_code: ErrorResponseErrorCode::AccessDenied,
                message: "Users cannot delete products".to_string(),
                details: None,
            }));
        }
        UserRole::Seller | UserRole::Admin => {}
    }

    let path = request.path;
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
                    message: "Sellers can only delete their own products".to_string(),
                    details: None,
                }));
            }
            _ => {}
        }
    }

    if client.archive_product(path.id).await? {
        Ok(PutProductsByIdResponse::NoContent)
    } else {
        Ok(PutProductsByIdResponse::NotFound(ErrorResponse {
            details: None,
            error_code: ErrorResponseErrorCode::ProductNotFound,
            message: "Product not found".to_string(),
        }))
    }
}
