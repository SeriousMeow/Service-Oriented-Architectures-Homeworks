use crate::api::*;
use crate::auth::JwtService;
use crate::db::Repository;
use crate::state::State;
use deadpool_postgres::GenericClient;

pub async fn handler(
    state: &State,
    request: PostAuthRefreshRequest,
) -> anyhow::Result<PostAuthLoginResponse> {
    let body = request.body;
    let mut client = state.db.get().await?;
    let tx = client.transaction().await?;

    let refresh_token = match tx.get_refresh_token(&body.refresh_token).await? {
        Some(rt) => rt,
        None => {
            return Ok(PostAuthLoginResponse::Unauthorized(ErrorResponse {
                error_code: ErrorResponseErrorCode::RefreshTokenInvalid,
                message: "Invalid or expired refresh token".to_string(),
                details: None,
            }));
        }
    };

    tx
        .delete_refresh_tokens_by_user(refresh_token.user_id)
        .await?;

    let user = match tx.get_user_by_id(refresh_token.user_id).await? {
        Some(u) => u,
        None => {
            return Ok(PostAuthLoginResponse::Unauthorized(ErrorResponse {
                error_code: ErrorResponseErrorCode::RefreshTokenInvalid,
                message: "User not found".to_string(),
                details: None,
            }));
        }
    };

    let jwt_service = JwtService::new(&state.config.jwt_secret);

    let access_token =
        jwt_service.generate_access_token(user.id, &user.email, user.role.clone())?;

    let new_refresh_token =
        jwt_service.generate_refresh_token(user.id, &user.email, user.role.clone())?;

    let expires_at = chrono::Utc::now() + chrono::Duration::days(30);
    tx
        .create_refresh_token(user.id, &new_refresh_token, expires_at)
        .await?;
    tx.commit().await?;

    Ok(PostAuthLoginResponse::Ok(AuthResponse {
        access_token,
        refresh_token: new_refresh_token,
        user_id: user.id,
        role: user.role,
    }))
}
