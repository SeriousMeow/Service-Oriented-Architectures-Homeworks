use crate::api::*;
use crate::auth::AuthContext;
use crate::db::Repository;
use crate::state::State;

pub async fn handle(
    _auth: &AuthContext,
    state: &State,
    request: GetProductsRequest,
) -> anyhow::Result<GetProductsResponse> {
    let page = request.query.page.unwrap_or(0);
    let size = request.query.size.unwrap_or(20);

    let offset = page * size;

    let status = request.query.status;
    let category = request.query.category;

    let client = state.db.get().await?;

    let products = client
        .get_products(status.clone(), category.clone(), size, offset)
        .await?
        .into_iter()
        .map(|product| product.into())
        .collect();
    let total = client.count_products(status, category).await?;

    Ok(GetProductsResponse::Ok(ProductsGetResponse {
        products,
        total_elements: total,
        current_page: ProductsGetResponseCurrentPage {
            page: Some(page),
            size: Some(size),
        },
    }))
}
