use crate::api::*;
use crate::auth::{JwtService, hash_password};
use crate::db::Repository;
use crate::state::State;
use deadpool_postgres::GenericClient;

fn internal_error(msg: String) -> PostAuthRegisterResponse {
    tracing::error!("Internal error: {}", msg);
    PostAuthRegisterResponse::BadRequest(ErrorResponse {
        error_code: ErrorResponseErrorCode::ValidationError,
        message: "Internal server error".to_string(),
        details: None,
    })
}

pub async fn handler(
    state: &State,
    request: PostAuthRegisterRequest,
) -> anyhow::Result<PostAuthRegisterResponse> {
    let body = request.body;
    let mut client = state.db.get().await?;
    let tx = client.transaction().await?;

    if let Some(_existing) = tx.get_user_by_email(&body.email).await? {
        return Ok(PostAuthRegisterResponse::Conflict(ErrorResponse {
            error_code: ErrorResponseErrorCode::UserAlreadyExists,
            message: "User with this email already exists".to_string(),
            details: None,
        }));
    }

    let password_hash = hash_password(&body.password)?;

    let user = tx
        .create_user(&body.email, &password_hash, body.role)
        .await?;

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

    Ok(PostAuthRegisterResponse::Created(AuthResponse {
        access_token,
        refresh_token,
        user_id: user.id,
        role: user.role,
    }))
}
