use crate::api::*;
use crate::auth::AuthContext;
use crate::db::Repository;
use crate::state::State;

pub async fn handle(
    _auth: &AuthContext,
    state: &State,
    request: GetProductsByIdRequest,
) -> anyhow::Result<GetProductsByIdResponse> {
    let path = request.path;
    let client = state.db.get().await?;

    match client.try_get_product(path.id).await? {
        None => Ok(GetProductsByIdResponse::NotFound(ErrorResponse {
            details: None,
            error_code: ErrorResponseErrorCode::ProductNotFound,
            message: "Product not found".to_string(),
        })),
        Some(product) => Ok(GetProductsByIdResponse::Ok(product.into())),
    }
}
