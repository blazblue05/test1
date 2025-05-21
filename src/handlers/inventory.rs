use actix_web::{web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use crate::db::{DbError, DbPool};
use crate::models::inventory_item::{InventoryItem, InventoryItemFilter, NewInventoryItem, UpdateInventoryItem};

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchQuery {
    pub category_id: Option<i64>,
    pub min_quantity: Option<i32>,
    pub max_quantity: Option<i32>,
    pub min_price: Option<f64>,
    pub max_price: Option<f64>,
    pub location: Option<String>,
    pub query: Option<String>,
}

pub async fn create_item(
    pool: web::Data<DbPool>,
    new_item: web::Json<NewInventoryItem>,
) -> impl Responder {
    match InventoryItem::create(&pool, new_item.into_inner()) {
        Ok(item_id) => {
            match InventoryItem::find_by_id(&pool, item_id, true) {
                Ok(item) => HttpResponse::Created().json(item),
                Err(_) => HttpResponse::InternalServerError().json(ErrorResponse {
                    error: "Item created but failed to retrieve".to_string(),
                }),
            }
        }
        Err(e) => {
            let error_message = match e {
                DbError::Sqlite(e) => {
                    if e.to_string().contains("UNIQUE constraint failed") {
                        "SKU already exists".to_string()
                    } else if e.to_string().contains("FOREIGN KEY constraint failed") {
                        "Invalid category ID".to_string()
                    } else {
                        format!("Database error: {}", e)
                    }
                }
                _ => format!("Error creating inventory item: {}", e),
            };
            
            HttpResponse::BadRequest().json(ErrorResponse {
                error: error_message,
            })
        }
    }
}

pub async fn get_item(
    pool: web::Data<DbPool>,
    path: web::Path<i64>,
) -> impl Responder {
    let item_id = path.into_inner();
    
    match InventoryItem::find_by_id(&pool, item_id, true) {
        Ok(item) => HttpResponse::Ok().json(item),
        Err(e) => {
            let mut status = match e {
                DbError::NotFound => HttpResponse::NotFound(),
                _ => HttpResponse::InternalServerError(),
            };
            
            status.json(ErrorResponse {
                error: format!("Error retrieving inventory item: {}", e),
            })
        }
    }
}

pub async fn update_item(
    pool: web::Data<DbPool>,
    path: web::Path<i64>,
    update: web::Json<UpdateInventoryItem>,
) -> impl Responder {
    let item_id = path.into_inner();
    
    match InventoryItem::update(&pool, item_id, update.into_inner()) {
        Ok(_) => {
            match InventoryItem::find_by_id(&pool, item_id, true) {
                Ok(item) => HttpResponse::Ok().json(item),
                Err(e) => HttpResponse::InternalServerError().json(ErrorResponse {
                    error: format!("Item updated but failed to retrieve: {}", e),
                }),
            }
        }
        Err(e) => {
            let mut status = match e {
                DbError::NotFound => HttpResponse::NotFound(),
                _ => HttpResponse::InternalServerError(),
            };
            
            status.json(ErrorResponse {
                error: format!("Error updating inventory item: {}", e),
            })
        }
    }
}

pub async fn delete_item(
    pool: web::Data<DbPool>,
    path: web::Path<i64>,
) -> impl Responder {
    let item_id = path.into_inner();
    
    match InventoryItem::delete(&pool, item_id) {
        Ok(_) => HttpResponse::NoContent().finish(),
        Err(e) => {
            let mut status = match e {
                DbError::NotFound => HttpResponse::NotFound(),
                _ => HttpResponse::InternalServerError(),
            };
            
            status.json(ErrorResponse {
                error: format!("Error deleting inventory item: {}", e),
            })
        }
    }
}

pub async fn list_items(
    pool: web::Data<DbPool>,
) -> impl Responder {
    match InventoryItem::list(&pool, true) {
        Ok(items) => HttpResponse::Ok().json(items),
        Err(e) => HttpResponse::InternalServerError().json(ErrorResponse {
            error: format!("Error listing inventory items: {}", e),
        }),
    }
}

pub async fn search_items(
    pool: web::Data<DbPool>,
    query: web::Query<SearchQuery>,
) -> impl Responder {
    let filter = InventoryItemFilter {
        category_id: query.category_id,
        min_quantity: query.min_quantity,
        max_quantity: query.max_quantity,
        min_price: query.min_price,
        max_price: query.max_price,
        location: query.location.clone(),
        search_query: query.query.clone(),
    };
    
    match InventoryItem::search(&pool, filter, true) {
        Ok(items) => HttpResponse::Ok().json(items),
        Err(e) => HttpResponse::InternalServerError().json(ErrorResponse {
            error: format!("Error searching inventory items: {}", e),
        }),
    }
}

pub async fn get_low_stock_items(
    pool: web::Data<DbPool>,
    query: web::Query<LowStockQuery>,
) -> impl Responder {
    let threshold = query.threshold.unwrap_or(10);
    
    match InventoryItem::get_low_stock_items(&pool, threshold) {
        Ok(items) => HttpResponse::Ok().json(items),
        Err(e) => HttpResponse::InternalServerError().json(ErrorResponse {
            error: format!("Error retrieving low stock items: {}", e),
        }),
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LowStockQuery {
    pub threshold: Option<i32>,
}