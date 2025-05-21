use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use rusqlite::{params, Result as SqliteResult, Row};
use crate::db::{DbError, DbPool, DbResult};
use crate::models::category::Category;

#[derive(Debug, Serialize, Deserialize)]
pub struct InventoryItem {
    pub id: Option<i64>,
    pub name: String,
    pub description: Option<String>,
    pub category_id: i64,
    pub quantity: i32,
    pub unit_price: f64,
    pub sku: Option<String>,
    pub location: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<Category>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NewInventoryItem {
    pub name: String,
    pub description: Option<String>,
    pub category_id: i64,
    pub quantity: i32,
    pub unit_price: f64,
    pub sku: Option<String>,
    pub location: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateInventoryItem {
    pub name: Option<String>,
    pub description: Option<String>,
    pub category_id: Option<i64>,
    pub quantity: Option<i32>,
    pub unit_price: Option<f64>,
    pub sku: Option<String>,
    pub location: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InventoryItemFilter {
    pub category_id: Option<i64>,
    pub min_quantity: Option<i32>,
    pub max_quantity: Option<i32>,
    pub min_price: Option<f64>,
    pub max_price: Option<f64>,
    pub location: Option<String>,
    pub search_query: Option<String>,
}

impl InventoryItem {
    pub fn from_row(row: &Row) -> SqliteResult<Self> {
        let created_at_str: String = row.get("created_at")?;
        let updated_at_str: String = row.get("updated_at")?;
        
        let created_at = DateTime::parse_from_rfc3339(&created_at_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());
        
        let updated_at = DateTime::parse_from_rfc3339(&updated_at_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());
        
        Ok(InventoryItem {
            id: row.get("id")?,
            name: row.get("name")?,
            description: row.get("description")?,
            category_id: row.get("category_id")?,
            quantity: row.get("quantity")?,
            unit_price: row.get("unit_price")?,
            sku: row.get("sku")?,
            location: row.get("location")?,
            created_at,
            updated_at,
            category: None,
        })
    }
    
    pub fn find_by_id(pool: &DbPool, id: i64, with_category: bool) -> DbResult<Self> {
        let conn = pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT id, name, description, category_id, quantity, unit_price, sku, location, created_at, updated_at 
             FROM inventory_items WHERE id = ?"
        )?;
        
        let mut item = stmt.query_row(params![id], |row| Self::from_row(row))?;
        
        if with_category {
            item.category = Some(Category::find_by_id(pool, item.category_id)?);
        }
        
        Ok(item)
    }
    
    pub fn create(pool: &DbPool, new_item: NewInventoryItem) -> DbResult<i64> {
        let conn = pool.get()?;
        
        let result = conn.execute(
            "INSERT INTO inventory_items (name, description, category_id, quantity, unit_price, sku, location) 
             VALUES (?, ?, ?, ?, ?, ?, ?)",
            params![
                new_item.name,
                new_item.description,
                new_item.category_id,
                new_item.quantity,
                new_item.unit_price,
                new_item.sku,
                new_item.location,
            ],
        )?;
        
        if result > 0 {
            Ok(conn.last_insert_rowid())
        } else {
            Err(DbError::NoRowsAffected)
        }
    }
    
    pub fn update(pool: &DbPool, id: i64, update: UpdateInventoryItem) -> DbResult<()> {
        let conn = pool.get()?;
        let mut query_parts = Vec::new();
        let mut params = Vec::new();
        
        if let Some(name) = update.name {
            query_parts.push("name = ?");
            params.push(name);
        }
        
        if let Some(description) = update.description {
            query_parts.push("description = ?");
            params.push(description);
        }
        
        if let Some(category_id) = update.category_id {
            query_parts.push("category_id = ?");
            params.push(category_id.to_string());
        }
        
        if let Some(quantity) = update.quantity {
            query_parts.push("quantity = ?");
            params.push(quantity.to_string());
        }
        
        if let Some(unit_price) = update.unit_price {
            query_parts.push("unit_price = ?");
            params.push(unit_price.to_string());
        }
        
        if let Some(sku) = update.sku {
            query_parts.push("sku = ?");
            params.push(sku);
        }
        
        if let Some(location) = update.location {
            query_parts.push("location = ?");
            params.push(location);
        }
        
        if query_parts.is_empty() {
            return Ok(());
        }
        
        query_parts.push("updated_at = CURRENT_TIMESTAMP");
        
        let query = format!(
            "UPDATE inventory_items SET {} WHERE id = ?",
            query_parts.join(", ")
        );
        
        params.push(id.to_string());
        
        let result = conn.execute(&query, rusqlite::params_from_iter(params))?;
        
        if result > 0 {
            Ok(())
        } else {
            Err(DbError::NoRowsAffected)
        }
    }
    
    pub fn delete(pool: &DbPool, id: i64) -> DbResult<()> {
        let conn = pool.get()?;
        let result = conn.execute("DELETE FROM inventory_items WHERE id = ?", params![id])?;
        
        if result > 0 {
            Ok(())
        } else {
            Err(DbError::NoRowsAffected)
        }
    }
    
    pub fn list(pool: &DbPool, with_category: bool) -> DbResult<Vec<Self>> {
        let conn = pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT id, name, description, category_id, quantity, unit_price, sku, location, created_at, updated_at 
             FROM inventory_items ORDER BY name"
        )?;
        
        let items_iter = stmt.query_map([], |row| Self::from_row(row))?;
        let mut items = Vec::new();
        
        for item_result in items_iter {
            match item_result {
                Ok(item) => {
                    let mut item_copy = item;
                    if with_category {
                        item_copy.category = Some(Category::find_by_id(pool, item_copy.category_id)?);
                    }
                    items.push(item_copy);
                },
                Err(e) => return Err(DbError::from(e)),
            }
        }
        
        Ok(items)
    }
    
    pub fn search(pool: &DbPool, filter: InventoryItemFilter, with_category: bool) -> DbResult<Vec<Self>> {
        let conn = pool.get()?;
        
        let mut conditions = Vec::new();
        let mut params = Vec::new();
        
        if let Some(category_id) = filter.category_id {
            conditions.push("category_id = ?");
            params.push(category_id.to_string());
        }
        
        if let Some(min_quantity) = filter.min_quantity {
            conditions.push("quantity >= ?");
            params.push(min_quantity.to_string());
        }
        
        if let Some(max_quantity) = filter.max_quantity {
            conditions.push("quantity <= ?");
            params.push(max_quantity.to_string());
        }
        
        if let Some(min_price) = filter.min_price {
            conditions.push("unit_price >= ?");
            params.push(min_price.to_string());
        }
        
        if let Some(max_price) = filter.max_price {
            conditions.push("unit_price <= ?");
            params.push(max_price.to_string());
        }
        
        if let Some(location) = filter.location {
            conditions.push("location LIKE ?");
            params.push(format!("%{}%", location));
        }
        
        if let Some(search_query) = filter.search_query {
            conditions.push("(name LIKE ? OR description LIKE ? OR sku LIKE ?)");
            let search_pattern = format!("%{}%", search_query);
            params.push(search_pattern.clone());
            params.push(search_pattern.clone());
            params.push(search_pattern);
        }
        
        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };
        
        let query = format!(
            "SELECT id, name, description, category_id, quantity, unit_price, sku, location, created_at, updated_at 
             FROM inventory_items {} ORDER BY name",
            where_clause
        );
        
        let mut stmt = conn.prepare(&query)?;
        
        let items_iter = stmt.query_map(rusqlite::params_from_iter(params), |row| {
            Self::from_row(row)
        })?;
        
        let mut items = Vec::new();
        
        for item_result in items_iter {
            match item_result {
                Ok(item) => {
                    let mut item_copy = item;
                    if with_category {
                        item_copy.category = Some(Category::find_by_id(pool, item_copy.category_id)?);
                    }
                    items.push(item_copy);
                },
                Err(e) => return Err(DbError::from(e)),
            }
        }
        
        Ok(items)
    }
    
    pub fn update_quantity(pool: &DbPool, id: i64, quantity_change: i32) -> DbResult<i32> {
        let conn = pool.get()?;
        
        // Get current quantity
        let mut stmt = conn.prepare("SELECT quantity FROM inventory_items WHERE id = ?")?;
        let current_quantity: i32 = stmt.query_row(params![id], |row| row.get(0))?;
        
        // Calculate new quantity
        let new_quantity = current_quantity + quantity_change;
        
        // Update quantity
        let result = conn.execute(
            "UPDATE inventory_items SET quantity = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
            params![new_quantity, id],
        )?;
        
        if result > 0 {
            Ok(new_quantity)
        } else {
            Err(DbError::NoRowsAffected)
        }
    }
    
    pub fn get_low_stock_items(pool: &DbPool, threshold: i32) -> DbResult<Vec<Self>> {
        let conn = pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT id, name, description, category_id, quantity, unit_price, sku, location, created_at, updated_at 
             FROM inventory_items 
             WHERE quantity <= ? 
             ORDER BY quantity ASC"
        )?;
        
        let items_iter = stmt.query_map(params![threshold], |row| Self::from_row(row))?;
        let mut items = Vec::new();
        
        for item_result in items_iter {
            match item_result {
                Ok(item) => {
                    let mut item_copy = item;
                    item_copy.category = Some(Category::find_by_id(pool, item_copy.category_id)?);
                    items.push(item_copy);
                },
                Err(e) => return Err(DbError::from(e)),
            }
        }
        
        Ok(items)
    }
}