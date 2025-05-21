use actix_web::{web, HttpResponse, Responder};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use rusqlite::params;
use crate::db::{DbError, DbPool};
use crate::models::category::Category;
use crate::models::inventory_item::InventoryItem;

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InventorySummary {
    pub total_items: i64,
    pub total_quantity: i64,
    pub total_value: f64,
    pub categories_count: i64,
    pub low_stock_count: i64,
    pub zero_stock_count: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CategorySummary {
    pub id: i64,
    pub name: String,
    pub items_count: i64,
    pub total_quantity: i64,
    pub total_value: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TransactionSummary {
    pub date: String,
    pub additions: i64,
    pub removals: i64,
    pub adjustments: i64,
    pub net_change: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DateRangeQuery {
    pub start_date: Option<String>,
    pub end_date: Option<String>,
}

pub async fn get_inventory_summary(
    pool: web::Data<DbPool>,
) -> impl Responder {
    let conn = match pool.get() {
        Ok(conn) => conn,
        Err(e) => {
            return HttpResponse::InternalServerError().json(ErrorResponse {
                error: format!("Database connection error: {}", e),
            });
        }
    };
    
    // Get total items count
    let total_items: i64 = match conn.query_row(
        "SELECT COUNT(*) FROM inventory_items",
        [],
        |row| row.get(0),
    ) {
        Ok(count) => count,
        Err(e) => {
            return HttpResponse::InternalServerError().json(ErrorResponse {
                error: format!("Error counting inventory items: {}", e),
            });
        }
    };
    
    // Get total quantity
    let total_quantity: i64 = match conn.query_row(
        "SELECT SUM(quantity) FROM inventory_items",
        [],
        |row| row.get::<_, Option<i64>>(0),
    ) {
        Ok(sum) => sum.unwrap_or(0),
        Err(e) => {
            return HttpResponse::InternalServerError().json(ErrorResponse {
                error: format!("Error summing inventory quantities: {}", e),
            });
        }
    };
    
    // Get total value
    let total_value: f64 = match conn.query_row(
        "SELECT SUM(quantity * unit_price) FROM inventory_items",
        [],
        |row| row.get::<_, Option<f64>>(0),
    ) {
        Ok(sum) => sum.unwrap_or(0.0),
        Err(e) => {
            return HttpResponse::InternalServerError().json(ErrorResponse {
                error: format!("Error calculating inventory value: {}", e),
            });
        }
    };
    
    // Get categories count
    let categories_count: i64 = match conn.query_row(
        "SELECT COUNT(*) FROM categories",
        [],
        |row| row.get(0),
    ) {
        Ok(count) => count,
        Err(e) => {
            return HttpResponse::InternalServerError().json(ErrorResponse {
                error: format!("Error counting categories: {}", e),
            });
        }
    };
    
    // Get low stock count (items with quantity <= 10)
    let low_stock_count: i64 = match conn.query_row(
        "SELECT COUNT(*) FROM inventory_items WHERE quantity > 0 AND quantity <= 10",
        [],
        |row| row.get(0),
    ) {
        Ok(count) => count,
        Err(e) => {
            return HttpResponse::InternalServerError().json(ErrorResponse {
                error: format!("Error counting low stock items: {}", e),
            });
        }
    };
    
    // Get zero stock count
    let zero_stock_count: i64 = match conn.query_row(
        "SELECT COUNT(*) FROM inventory_items WHERE quantity = 0",
        [],
        |row| row.get(0),
    ) {
        Ok(count) => count,
        Err(e) => {
            return HttpResponse::InternalServerError().json(ErrorResponse {
                error: format!("Error counting zero stock items: {}", e),
            });
        }
    };
    
    let summary = InventorySummary {
        total_items,
        total_quantity,
        total_value,
        categories_count,
        low_stock_count,
        zero_stock_count,
    };
    
    HttpResponse::Ok().json(summary)
}

pub async fn get_category_summary(
    pool: web::Data<DbPool>,
) -> impl Responder {
    let conn = match pool.get() {
        Ok(conn) => conn,
        Err(e) => {
            return HttpResponse::InternalServerError().json(ErrorResponse {
                error: format!("Database connection error: {}", e),
            });
        }
    };
    
    let mut stmt = match conn.prepare(
        "SELECT 
            c.id, 
            c.name, 
            COUNT(i.id) as items_count, 
            SUM(i.quantity) as total_quantity, 
            SUM(i.quantity * i.unit_price) as total_value
         FROM categories c
         LEFT JOIN inventory_items i ON c.id = i.category_id
         GROUP BY c.id
         ORDER BY c.name"
    ) {
        Ok(stmt) => stmt,
        Err(e) => {
            return HttpResponse::InternalServerError().json(ErrorResponse {
                error: format!("Error preparing category summary query: {}", e),
            });
        }
    };
    
    let category_summaries = match stmt.query_map([], |row| {
        Ok(CategorySummary {
            id: row.get(0)?,
            name: row.get(1)?,
            items_count: row.get(2)?,
            total_quantity: row.get(3).unwrap_or(0),
            total_value: row.get(4).unwrap_or(0.0),
        })
    }) {
        Ok(rows) => {
            let mut summaries = Vec::new();
            for row_result in rows {
                match row_result {
                    Ok(summary) => summaries.push(summary),
                    Err(e) => {
                        return HttpResponse::InternalServerError().json(ErrorResponse {
                            error: format!("Error processing category summary row: {}", e),
                        });
                    }
                }
            }
            summaries
        }
        Err(e) => {
            return HttpResponse::InternalServerError().json(ErrorResponse {
                error: format!("Error executing category summary query: {}", e),
            });
        }
    };
    
    HttpResponse::Ok().json(category_summaries)
}

pub async fn get_transaction_history(
    pool: web::Data<DbPool>,
    query: web::Query<DateRangeQuery>,
) -> impl Responder {
    let conn = match pool.get() {
        Ok(conn) => conn,
        Err(e) => {
            return HttpResponse::InternalServerError().json(ErrorResponse {
                error: format!("Database connection error: {}", e),
            });
        }
    };
    
    let start_date = query.start_date.clone().unwrap_or_else(|| {
        // Default to 30 days ago
        let thirty_days_ago = Utc::now() - chrono::Duration::days(30);
        thirty_days_ago.format("%Y-%m-%d").to_string()
    });
    
    let end_date = query.end_date.clone().unwrap_or_else(|| {
        // Default to today
        Utc::now().format("%Y-%m-%d").to_string()
    });
    
    let query_sql = 
        "SELECT 
            date(transaction_date) as date,
            SUM(CASE WHEN transaction_type = 'addition' THEN quantity ELSE 0 END) as additions,
            SUM(CASE WHEN transaction_type = 'removal' THEN quantity ELSE 0 END) as removals,
            SUM(CASE WHEN transaction_type = 'adjustment' THEN quantity ELSE 0 END) as adjustments,
            SUM(CASE 
                WHEN transaction_type = 'addition' THEN quantity 
                WHEN transaction_type = 'removal' THEN -quantity 
                WHEN transaction_type = 'adjustment' THEN quantity 
                ELSE 0 
            END) as net_change
         FROM inventory_transactions
         WHERE date(transaction_date) BETWEEN ? AND ?
         GROUP BY date(transaction_date)
         ORDER BY date(transaction_date)";
    
    let mut stmt = match conn.prepare(query_sql) {
        Ok(stmt) => stmt,
        Err(e) => {
            return HttpResponse::InternalServerError().json(ErrorResponse {
                error: format!("Error preparing transaction history query: {}", e),
            });
        }
    };
    
    let transaction_summaries = match stmt.query_map(params![start_date, end_date], |row| {
        Ok(TransactionSummary {
            date: row.get(0)?,
            additions: row.get(1)?,
            removals: row.get(2)?,
            adjustments: row.get(3)?,
            net_change: row.get(4)?,
        })
    }) {
        Ok(rows) => {
            let mut summaries = Vec::new();
            for row_result in rows {
                match row_result {
                    Ok(summary) => summaries.push(summary),
                    Err(e) => {
                        return HttpResponse::InternalServerError().json(ErrorResponse {
                            error: format!("Error processing transaction summary row: {}", e),
                        });
                    }
                }
            }
            summaries
        }
        Err(e) => {
            return HttpResponse::InternalServerError().json(ErrorResponse {
                error: format!("Error executing transaction history query: {}", e),
            });
        }
    };
    
    HttpResponse::Ok().json(transaction_summaries)
}