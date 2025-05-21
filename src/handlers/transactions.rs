use actix_web::{web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use crate::db::{DbError, DbPool};
use crate::models::transaction::{NewTransaction, Transaction};

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
}

pub async fn create_transaction(
    pool: web::Data<DbPool>,
    new_transaction: web::Json<NewTransaction>,
) -> impl Responder {
    match Transaction::create(&pool, new_transaction.into_inner()) {
        Ok(transaction_id) => {
            match Transaction::find_by_id(&pool, transaction_id, true) {
                Ok(transaction) => HttpResponse::Created().json(transaction),
                Err(_) => HttpResponse::InternalServerError().json(ErrorResponse {
                    error: "Transaction created but failed to retrieve".to_string(),
                }),
            }
        }
        Err(e) => {
            let error_message = match e {
                DbError::Sqlite(e) => {
                    if e.to_string().contains("FOREIGN KEY constraint failed") {
                        "Invalid item ID or user ID".to_string()
                    } else {
                        format!("Database error: {}", e)
                    }
                }
                _ => format!("Error creating transaction: {}", e),
            };
            
            HttpResponse::BadRequest().json(ErrorResponse {
                error: error_message,
            })
        }
    }
}

pub async fn get_transaction(
    pool: web::Data<DbPool>,
    path: web::Path<i64>,
) -> impl Responder {
    let transaction_id = path.into_inner();
    
    match Transaction::find_by_id(&pool, transaction_id, true) {
        Ok(transaction) => HttpResponse::Ok().json(transaction),
        Err(e) => {
            let mut status = match e {
                DbError::NotFound => HttpResponse::NotFound(),
                _ => HttpResponse::InternalServerError(),
            };
            
            status.json(ErrorResponse {
                error: format!("Error retrieving transaction: {}", e),
            })
        }
    }
}

pub async fn list_item_transactions(
    pool: web::Data<DbPool>,
    path: web::Path<i64>,
) -> impl Responder {
    let item_id = path.into_inner();
    
    match Transaction::list_by_item(&pool, item_id, true) {
        Ok(transactions) => HttpResponse::Ok().json(transactions),
        Err(e) => HttpResponse::InternalServerError().json(ErrorResponse {
            error: format!("Error listing item transactions: {}", e),
        }),
    }
}

pub async fn list_user_transactions(
    pool: web::Data<DbPool>,
    path: web::Path<i64>,
) -> impl Responder {
    let user_id = path.into_inner();
    
    match Transaction::list_by_user(&pool, user_id, true) {
        Ok(transactions) => HttpResponse::Ok().json(transactions),
        Err(e) => HttpResponse::InternalServerError().json(ErrorResponse {
            error: format!("Error listing user transactions: {}", e),
        }),
    }
}

pub async fn list_recent_transactions(
    pool: web::Data<DbPool>,
    query: web::Query<RecentTransactionsQuery>,
) -> impl Responder {
    let limit = query.limit.unwrap_or(20);
    
    match Transaction::list_recent(&pool, limit, true) {
        Ok(transactions) => HttpResponse::Ok().json(transactions),
        Err(e) => HttpResponse::InternalServerError().json(ErrorResponse {
            error: format!("Error listing recent transactions: {}", e),
        }),
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RecentTransactionsQuery {
    pub limit: Option<i64>,
}