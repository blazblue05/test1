use actix_web::{web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use crate::db::{DbError, DbPool};
use crate::models::category::{Category, NewCategory, UpdateCategory};

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchQuery {
    pub query: String,
}

pub async fn create_category(
    pool: web::Data<DbPool>,
    new_category: web::Json<NewCategory>,
) -> impl Responder {
    match Category::create(&pool, new_category.into_inner()) {
        Ok(category_id) => {
            match Category::find_by_id(&pool, category_id) {
                Ok(category) => HttpResponse::Created().json(category),
                Err(_) => HttpResponse::InternalServerError().json(ErrorResponse {
                    error: "Category created but failed to retrieve".to_string(),
                }),
            }
        }
        Err(e) => {
            let error_message = match e {
                DbError::Sqlite(e) => {
                    if e.to_string().contains("UNIQUE constraint failed") {
                        "Category name already exists".to_string()
                    } else {
                        format!("Database error: {}", e)
                    }
                }
                _ => format!("Error creating category: {}", e),
            };
            
            HttpResponse::BadRequest().json(ErrorResponse {
                error: error_message,
            })
        }
    }
}

pub async fn get_category(
    pool: web::Data<DbPool>,
    path: web::Path<i64>,
) -> impl Responder {
    let category_id = path.into_inner();
    
    match Category::find_by_id(&pool, category_id) {
        Ok(category) => HttpResponse::Ok().json(category),
        Err(e) => {
            let mut status = match e {
                DbError::NotFound => HttpResponse::NotFound(),
                _ => HttpResponse::InternalServerError(),
            };
            
            status.json(ErrorResponse {
                error: format!("Error retrieving category: {}", e),
            })
        }
    }
}

pub async fn update_category(
    pool: web::Data<DbPool>,
    path: web::Path<i64>,
    update: web::Json<UpdateCategory>,
) -> impl Responder {
    let category_id = path.into_inner();
    
    match Category::update(&pool, category_id, update.into_inner()) {
        Ok(_) => {
            match Category::find_by_id(&pool, category_id) {
                Ok(category) => HttpResponse::Ok().json(category),
                Err(e) => HttpResponse::InternalServerError().json(ErrorResponse {
                    error: format!("Category updated but failed to retrieve: {}", e),
                }),
            }
        }
        Err(e) => {
            let mut status = match e {
                DbError::NotFound => HttpResponse::NotFound(),
                _ => HttpResponse::InternalServerError(),
            };
            
            status.json(ErrorResponse {
                error: format!("Error updating category: {}", e),
            })
        }
    }
}

pub async fn delete_category(
    pool: web::Data<DbPool>,
    path: web::Path<i64>,
) -> impl Responder {
    let category_id = path.into_inner();
    
    match Category::delete(&pool, category_id) {
        Ok(_) => HttpResponse::NoContent().finish(),
        Err(e) => {
            let mut status = match e {
                DbError::NotFound => HttpResponse::NotFound(),
                _ => HttpResponse::InternalServerError(),
            };
            
            status.json(ErrorResponse {
                error: format!("Error deleting category: {}", e),
            })
        }
    }
}

pub async fn list_categories(
    pool: web::Data<DbPool>,
) -> impl Responder {
    match Category::list(&pool) {
        Ok(categories) => HttpResponse::Ok().json(categories),
        Err(e) => HttpResponse::InternalServerError().json(ErrorResponse {
            error: format!("Error listing categories: {}", e),
        }),
    }
}

pub async fn search_categories(
    pool: web::Data<DbPool>,
    query: web::Query<SearchQuery>,
) -> impl Responder {
    match Category::search(&pool, &query.query) {
        Ok(categories) => HttpResponse::Ok().json(categories),
        Err(e) => HttpResponse::InternalServerError().json(ErrorResponse {
            error: format!("Error searching categories: {}", e),
        }),
    }
}