use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use rusqlite::{params, Result as SqliteResult, Row};
use crate::db::{DbError, DbPool, DbResult};

#[derive(Debug, Serialize, Deserialize)]
pub struct Category {
    pub id: Option<i64>,
    pub name: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NewCategory {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateCategory {
    pub name: Option<String>,
    pub description: Option<String>,
}

impl Category {
    pub fn from_row(row: &Row) -> SqliteResult<Self> {
        let created_at_str: String = row.get("created_at")?;
        let updated_at_str: String = row.get("updated_at")?;
        
        let created_at = DateTime::parse_from_rfc3339(&created_at_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());
        
        let updated_at = DateTime::parse_from_rfc3339(&updated_at_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());
        
        Ok(Category {
            id: row.get("id")?,
            name: row.get("name")?,
            description: row.get("description")?,
            created_at,
            updated_at,
        })
    }
    
    pub fn find_by_id(pool: &DbPool, id: i64) -> DbResult<Self> {
        let conn = pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT id, name, description, created_at, updated_at FROM categories WHERE id = ?"
        )?;
        
        stmt.query_row(params![id], |row| Self::from_row(row))
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => DbError::NotFound,
                _ => DbError::from(e),
            })
    }
    
    pub fn create(pool: &DbPool, new_category: NewCategory) -> DbResult<i64> {
        let conn = pool.get()?;
        
        let result = conn.execute(
            "INSERT INTO categories (name, description) VALUES (?, ?)",
            params![new_category.name, new_category.description],
        )?;
        
        if result > 0 {
            Ok(conn.last_insert_rowid())
        } else {
            Err(DbError::NoRowsAffected)
        }
    }
    
    pub fn update(pool: &DbPool, id: i64, update: UpdateCategory) -> DbResult<()> {
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
        
        if query_parts.is_empty() {
            return Ok(());
        }
        
        query_parts.push("updated_at = CURRENT_TIMESTAMP");
        
        let query = format!(
            "UPDATE categories SET {} WHERE id = ?",
            query_parts.join(", ")
        );
        
        params.push(id.to_string());
        
        let result = conn.execute(&query, rusqlite::params_from_iter(params))?;
        
        if result > 0 {
            Ok(())
        } else {
            Err(DbError::NotFound)
        }
    }
    
    pub fn delete(pool: &DbPool, id: i64) -> DbResult<()> {
        let conn = pool.get()?;
        let result = conn.execute("DELETE FROM categories WHERE id = ?", params![id])?;
        
        if result > 0 {
            Ok(())
        } else {
            Err(DbError::NotFound)
        }
    }
    
    pub fn list(pool: &DbPool) -> DbResult<Vec<Self>> {
        let conn = pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT id, name, description, created_at, updated_at 
             FROM categories ORDER BY name"
        )?;
        
        let categories_iter = stmt.query_map([], |row| Self::from_row(row))?;
        let mut categories = Vec::new();
        
        for category_result in categories_iter {
            match category_result {
                Ok(category) => categories.push(category),
                Err(e) => return Err(DbError::from(e)),
            };
        }
        
        Ok(categories)
    }
    
    pub fn search(pool: &DbPool, search_query: &str) -> DbResult<Vec<Self>> {
        let conn = pool.get()?;
        let search_query = format!("%{}%", search_query);
        
        let mut stmt = conn.prepare(
            "SELECT id, name, description, created_at, updated_at 
             FROM categories 
             WHERE name LIKE ? OR description LIKE ? 
             ORDER BY name"
        )?;
        
        let categories_iter = stmt.query_map(params![search_query, search_query], |row| {
            Self::from_row(row)
        })?;
        
        let mut categories = Vec::new();
        
        for category_result in categories_iter {
            match category_result {
                Ok(category) => categories.push(category),
                Err(e) => return Err(DbError::from(e)),
            };
        }
        
        Ok(categories)
    }
}