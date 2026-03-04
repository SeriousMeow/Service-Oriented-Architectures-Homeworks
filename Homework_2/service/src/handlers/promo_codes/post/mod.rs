use crate::api::*;
use crate::auth::AuthContext;
use crate::db::Repository;
use crate::state::State;
use rust_decimal::Decimal;
use std::str::FromStr;

pub async fn handle(
    auth: &AuthContext,
    state: &State,
    request: PostPromoCodesRequest,
) -> anyhow::Result<PostPromoCodesResponse> {
    match auth.role {
        UserRole::User => {
            return Ok(PostPromoCodesResponse::Forbidden(ErrorResponse {
                error_code: ErrorResponseErrorCode::AccessDenied,
                message: "Users cannot create promo codes".to_string(),
                details: None,
            }));
        }
        UserRole::Seller | UserRole::Admin => {}
    }

    let body = request.body;
    let client = state.db.get().await?;

    let result = client
        .create_promo_code(
            body.code.clone(),
            body.discount_type,
            Decimal::from_str(&body.discount_value.to_string())?,
            Decimal::from_str(&body.min_order_amount.to_string())?,
            body.max_uses as i32,
            body.current_uses.unwrap_or(0) as i32,
            body.valid_from,
            body.valid_until,
            body.active.unwrap_or(true),
        )
        .await;

    match result {
        Ok(promo_code) => Ok(PostPromoCodesResponse::Created(PromoCodeResponse {
            id: promo_code.id,
            code: promo_code.code,
            discount_type: promo_code.discount_type,
            discount_value: promo_code.discount_value.to_string(),
            min_order_amount: promo_code.min_order_amount.to_string(),
            max_uses: promo_code.max_uses as i64,
            current_uses: promo_code.current_uses as i64,
            valid_from: promo_code.valid_from,
            valid_until: promo_code.valid_until,
            active: promo_code.active,
        })),
        Err(e) => {
            let error_msg = e.to_string();
            if error_msg.contains("duplicate key value") || error_msg.contains("promo_codes_code_key") {
                Ok(PostPromoCodesResponse::BadRequest(ErrorResponse {
                    error_code: ErrorResponseErrorCode::ValidationError,
                    message: format!("Promo code '{}' already exists", body.code),
                    details: None,
                }))
            } else if error_msg.contains("check_valid_dates") {
                Ok(PostPromoCodesResponse::BadRequest(ErrorResponse {
                    error_code: ErrorResponseErrorCode::ValidationError,
                    message: "valid_until must be greater than valid_from".to_string(),
                    details: None,
                }))
            } else {
                Err(e)
            }
        }
    }
}
