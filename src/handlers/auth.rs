use actix_web::{web, HttpResponse, Responder};
use chrono::Duration;
use serde::{Deserialize, Serialize};
use crate::auth::jwt;
use crate::auth::password;
use crate::db::DbPool;
use crate::models::user::{LoginCredentials, User};

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthResponse {
    pub token: String,
    pub user_id: i64,
    pub username: String,
    pub role: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
}

pub async fn login(
    pool: web::Data<DbPool>,
    credentials: web::Json<LoginCredentials>,
    config: web::Data<crate::config::Config>,
) -> impl Responder {
    // Find user by username
    let user = match User::find_by_username(&pool, &credentials.username) {
        Ok(user) => user,
        Err(_) => {
            return HttpResponse::Unauthorized().json(ErrorResponse {
                error: "Invalid username or password".to_string(),
            });
        }
    };
    
    // Verify password
    let password_verified = match password::verify_password(&credentials.password, &user.password_hash) {
        Ok(verified) => verified,
        Err(_) => {
            return HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Failed to verify password".to_string(),
            });
        }
    };
    
    if !password_verified {
        return HttpResponse::Unauthorized().json(ErrorResponse {
            error: "Invalid username or password".to_string(),
        });
    }
    
    // Generate JWT token
    let token = match jwt::create_token(
        user.id.unwrap(),
        &user.username,
        &user.role,
        config.jwt_secret.as_bytes(),
        Duration::seconds(config.jwt_expiration),
    ) {
        Ok(token) => token,
        Err(_) => {
            return HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Failed to generate token".to_string(),
            });
        }
    };
    
    HttpResponse::Ok().json(AuthResponse {
        token,
        user_id: user.id.unwrap(),
        username: user.username,
        role: user.role.to_string(),
    })
}