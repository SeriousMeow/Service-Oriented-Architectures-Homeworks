use anyhow::Result;
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};

use crate::api::UserRole;

const ACCESS_TOKEN_EXPIRY_MINUTES: i64 = 30;
const REFRESH_TOKEN_EXPIRY_DAYS: i64 = 30;

#[derive(Clone, Debug)]
pub struct AuthContext {
    pub user_id: i64,
    pub email: String,
    pub role: UserRole,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub user_id: i64,
    pub role: UserRole,
    pub exp: i64,
    pub iat: i64,
}

pub struct JwtService {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
}

impl JwtService {
    pub fn new(secret: &str) -> Self {
        Self {
            encoding_key: EncodingKey::from_secret(secret.as_bytes()),
            decoding_key: DecodingKey::from_secret(secret.as_bytes()),
        }
    }

    pub fn generate_access_token(
        &self,
        user_id: i64,
        email: &str,
        role: UserRole,
    ) -> Result<String> {
        let now = chrono::Utc::now().timestamp();
        let exp = now + ACCESS_TOKEN_EXPIRY_MINUTES * 60;

        let claims = Claims {
            sub: email.to_string(),
            user_id,
            role: role,
            exp,
            iat: now,
        };

        Ok(encode(&Header::default(), &claims, &self.encoding_key)?)
    }

    pub fn generate_refresh_token(
        &self,
        user_id: i64,
        email: &str,
        role: UserRole,
    ) -> Result<String> {
        let now = chrono::Utc::now().timestamp();
        let exp = now + REFRESH_TOKEN_EXPIRY_DAYS * 24 * 60 * 60;

        let claims = Claims {
            sub: email.to_string(),
            user_id,
            role: role,
            exp,
            iat: now,
        };

        Ok(encode(&Header::default(), &claims, &self.encoding_key)?)
    }

    pub fn validate_token(&self, token: &str) -> Result<Claims> {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.validate_exp = true;

        let token_data = decode::<Claims>(token, &self.decoding_key, &validation)?;
        Ok(token_data.claims)
    }
}

pub fn hash_password(password: &str) -> Result<String> {
    Ok(bcrypt::hash(password, bcrypt::DEFAULT_COST)?)
}

pub fn verify_password(password: &str, hash: &str) -> Result<bool> {
    Ok(bcrypt::verify(password, hash)?)
}
