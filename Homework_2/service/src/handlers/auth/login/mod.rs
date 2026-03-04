use crate::api::*;
use crate::auth::{JwtService, verify_password};
use crate::db::Repository;
use crate::state::State;
use deadpool_postgres::GenericClient;

pub async fn handler(
    state: &State,
    request: PostAuthLoginRequest,
) -> anyhow::Result<PostAuthLoginResponse> {
    let body = request.body;
    let mut client = state.db.get().await?;
    let tx = client.transaction().await?;

    let user = match tx.get_user_by_email(&body.email).await? {
        Some(u) => u,
        None => {
            return Ok(PostAuthLoginResponse::Unauthorized(ErrorResponse {
                error_code: ErrorResponseErrorCode::InvalidCredentials,
                message: "Invalid email or password".to_string(),
                details: None,
            }));
        }
    };

    let password_valid = verify_password(&body.password, &user.password_hash)?;

    if !password_valid {
        return Ok(PostAuthLoginResponse::Unauthorized(ErrorResponse {
            error_code: ErrorResponseErrorCode::InvalidCredentials,
            message: "Invalid email or password".to_string(),
            details: None,
        }));
    }

    let jwt_service = JwtService::new(&state.config.jwt_secret);

    let access_token =
        jwt_service.generate_access_token(user.id, &user.email, user.role.clone())?;
    let refresh_token =
        jwt_service.generate_refresh_token(user.id, &user.email, user.role.clone())?;

    let expires_at = chrono::Utc::now() + chrono::Duration::days(30);
    tx
        .delete_refresh_tokens_by_user(user.id)
        .await?;
    tx
        .create_refresh_token(user.id, &refresh_token, expires_at)
        .await?;
    tx.commit().await?;

    Ok(PostAuthLoginResponse::Ok(AuthResponse {
        access_token,
        refresh_token,
        user_id: user.id,
        role: user.role,
    }))
}
