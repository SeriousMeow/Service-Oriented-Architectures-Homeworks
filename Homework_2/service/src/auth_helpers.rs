use crate::api::*;
use crate::api::UserRole;
use crate::auth::{AuthContext, JwtService};
use crate::state::State;

pub fn verify_jwt(
    authorization: Option<String>,
    state: &State,
) -> Result<AuthContext, ErrorResponse> {
    let auth_header = authorization.as_deref().ok_or_else(|| ErrorResponse {
        error_code: ErrorResponseErrorCode::TokenInvalid,
        message: "Missing authorization header".to_string(),
        details: None,
    })?;

    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or_else(|| ErrorResponse {
            error_code: ErrorResponseErrorCode::TokenInvalid,
            message: "Invalid authorization header format".to_string(),
            details: None,
        })?;

    let jwt_service = JwtService::new(&state.config.jwt_secret);

    let claims = jwt_service
        .validate_token(token)
        .map_err(|_| ErrorResponse {
            error_code: ErrorResponseErrorCode::TokenExpired,
            message: "Invalid or expired token".to_string(),
            details: None,
        })?;

    Ok(AuthContext {
        user_id: claims.user_id,
        email: claims.sub,
        role: claims.role,
    })
}

pub fn extract_auth_from_header(
    authorization: Option<&str>,
    state: &State,
) -> Result<AuthContext, ErrorResponse> {
    let auth_header = authorization.ok_or_else(|| ErrorResponse {
        error_code: ErrorResponseErrorCode::TokenInvalid,
        message: "Missing authorization header".to_string(),
        details: None,
    })?;

    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or_else(|| ErrorResponse {
            error_code: ErrorResponseErrorCode::TokenInvalid,
            message: "Invalid authorization header format".to_string(),
            details: None,
        })?;

    let jwt_service = JwtService::new(&state.config.jwt_secret);

    let claims = jwt_service
        .validate_token(token)
        .map_err(|_| ErrorResponse {
            error_code: ErrorResponseErrorCode::TokenExpired,
            message: "Invalid or expired token".to_string(),
            details: None,
        })?;

    Ok(AuthContext {
        user_id: claims.user_id,
        email: claims.sub,
        role: claims.role,
    })
}

pub fn check_role(context: &AuthContext, allowed_roles: &[UserRole]) -> Result<(), ErrorResponse> {
    if !allowed_roles.contains(&context.role) {
        return Err(ErrorResponse {
            error_code: ErrorResponseErrorCode::AccessDenied,
            message: "Insufficient permissions".to_string(),
            details: None,
        });
    }
    Ok(())
}
