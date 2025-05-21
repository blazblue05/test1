use chrono::{DateTime, Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use crate::models::user::UserRole;

#[derive(Debug, Serialize, Deserialize)]
#[derive(Clone)]
pub struct Claims {
    pub sub: String,        // Subject (user ID)
    pub username: String,   // Username
    pub role: String,       // User role
    pub exp: usize,         // Expiration time (as UTC timestamp)
    pub iat: usize,         // Issued at (as UTC timestamp)
}

#[derive(Debug, Error)]
pub enum JwtError {
    #[error("Failed to create token: {0}")]
    TokenCreationError(String),
    #[error("Failed to validate token: {0}")]
    TokenValidationError(String),
    #[error("Token expired")]
    TokenExpired,
    #[error("Invalid token")]
    InvalidToken,
}

pub fn create_token(
    user_id: i64,
    username: &str,
    role: &UserRole,
    secret: &[u8],
    expiration: Duration,
) -> Result<String, JwtError> {
    let now = Utc::now();
    let expiration_time = now + expiration;
    
    let claims = Claims {
        sub: user_id.to_string(),
        username: username.to_string(),
        role: role.to_string(),
        iat: now.timestamp() as usize,
        exp: expiration_time.timestamp() as usize,
    };
    
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret),
    )
    .map_err(|e| JwtError::TokenCreationError(e.to_string()))
}

pub fn validate_token(token: &str, secret: &[u8]) -> Result<Claims, JwtError> {
    decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret),
        &Validation::default(),
    )
    .map(|data| data.claims)
    .map_err(|e| {
        match e.kind() {
            jsonwebtoken::errors::ErrorKind::ExpiredSignature => JwtError::TokenExpired,
            jsonwebtoken::errors::ErrorKind::InvalidToken => JwtError::InvalidToken,
            _ => JwtError::TokenValidationError(e.to_string()),
        }
    })
}