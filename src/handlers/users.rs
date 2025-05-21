use actix_web::{web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use crate::auth::password;
use crate::db::{DbError, DbPool};
use crate::models::user::{NewUser, UpdateUser, User, UserRole};

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
}

pub async fn create_user(
    pool: web::Data<DbPool>,
    new_user: web::Json<NewUser>,
) -> impl Responder {
    // Hash the password
    let password_hash = match password::hash_password(&new_user.password) {
        Ok(hash) => hash,
        Err(_) => {
            return HttpResponse::InternalServerError().json(ErrorResponse {
                error: "Failed to hash password".to_string(),
            });
        }
    };
    
    // Create the user
    match User::create(&pool, new_user.into_inner(), password_hash) {
        Ok(user_id) => {
            match User::find_by_id(&pool, user_id) {
                Ok(user) => HttpResponse::Created().json(user),
                Err(_) => HttpResponse::InternalServerError().json(ErrorResponse {
                    error: "User created but failed to retrieve".to_string(),
                }),
            }
        }
        Err(e) => {
            let error_message = match e {
                DbError::Sqlite(e) => {
                    if e.to_string().contains("UNIQUE constraint failed") {
                        "Username or email already exists".to_string()
                    } else {
                        format!("Database error: {}", e)
                    }
                }
                _ => format!("Error creating user: {}", e),
            };
            
            HttpResponse::BadRequest().json(ErrorResponse {
                error: error_message,
            })
        }
    }
}

pub async fn get_user(
    pool: web::Data<DbPool>,
    path: web::Path<i64>,
) -> impl Responder {
    let user_id = path.into_inner();
    
    match User::find_by_id(&pool, user_id) {
        Ok(user) => HttpResponse::Ok().json(user),
        Err(e) => {
            let mut status = match e {
                DbError::NotFound => HttpResponse::NotFound(),
                _ => HttpResponse::InternalServerError(),
            };
            
            status.json(ErrorResponse {
                error: format!("Error retrieving user: {}", e),
            })
        }
    }
}

pub async fn update_user(
    pool: web::Data<DbPool>,
    path: web::Path<i64>,
    update: web::Json<UpdateUser>,
) -> impl Responder {
    let user_id = path.into_inner();
    
    match User::update(&pool, user_id, update.into_inner()) {
        Ok(_) => {
            match User::find_by_id(&pool, user_id) {
                Ok(user) => HttpResponse::Ok().json(user),
                Err(e) => HttpResponse::InternalServerError().json(ErrorResponse {
                    error: format!("User updated but failed to retrieve: {}", e),
                }),
            }
        }
        Err(e) => {
            let mut status = match e {
                DbError::NotFound => HttpResponse::NotFound(),
                _ => HttpResponse::InternalServerError(),
            };
            
            status.json(ErrorResponse {
                error: format!("Error updating user: {}", e),
            })
        }
    }
}

pub async fn delete_user(
    pool: web::Data<DbPool>,
    path: web::Path<i64>,
) -> impl Responder {
    let user_id = path.into_inner();
    
    match User::delete(&pool, user_id) {
        Ok(_) => HttpResponse::NoContent().finish(),
        Err(e) => {
            let mut status = match e {
                DbError::NotFound => HttpResponse::NotFound(),
                _ => HttpResponse::InternalServerError(),
            };
            
            status.json(ErrorResponse {
                error: format!("Error deleting user: {}", e),
            })
        }
    }
}

pub async fn list_users(
    pool: web::Data<DbPool>,
) -> impl Responder {
    match User::list(&pool) {
        Ok(users) => HttpResponse::Ok().json(users),
        Err(e) => HttpResponse::InternalServerError().json(ErrorResponse {
            error: format!("Error listing users: {}", e),
        }),
    }
}