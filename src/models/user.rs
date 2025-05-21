use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use rusqlite::{params, Connection, Result as SqliteResult, Row};
use crate::db::{DbError, DbPool, DbResult};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum UserRole {
    Admin,
    Manager,
    User,
}

impl UserRole {
    pub fn from_str(role: &str) -> Option<Self> {
        match role.to_lowercase().as_str() {
            "admin" => Some(UserRole::Admin),
            "manager" => Some(UserRole::Manager),
            "user" => Some(UserRole::User),
            _ => None,
        }
    }
    
    pub fn to_string(&self) -> String {
        match self {
            UserRole::Admin => "admin".to_string(),
            UserRole::Manager => "manager".to_string(),
            UserRole::User => "user".to_string(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    pub id: Option<i64>,
    pub username: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub email: String,
    pub role: UserRole,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NewUser {
    pub username: String,
    pub password: String,
    pub email: String,
    pub role: UserRole,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateUser {
    pub username: Option<String>,
    pub email: Option<String>,
    pub role: Option<UserRole>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginCredentials {
    pub username: String,
    pub password: String,
}

impl User {
    pub fn from_row(row: &Row) -> SqliteResult<Self> {
        let role_str: String = row.get("role")?;
        let role = UserRole::from_str(&role_str).unwrap_or(UserRole::User);
        
        let created_at_str: String = row.get("created_at")?;
        let updated_at_str: String = row.get("updated_at")?;
        
        let created_at = DateTime::parse_from_rfc3339(&created_at_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());
        
        let updated_at = DateTime::parse_from_rfc3339(&updated_at_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());
        
        Ok(User {
            id: row.get("id")?,
            username: row.get("username")?,
            password_hash: row.get("password_hash")?,
            email: row.get("email")?,
            role,
            created_at,
            updated_at,
        })
    }
    
    pub fn find_by_id(pool: &DbPool, id: i64) -> DbResult<Self> {
        let conn = pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT id, username, password_hash, email, role, created_at, updated_at 
             FROM users WHERE id = ?"
        )?;
        
        let user = stmt.query_row(params![id], |row| Self::from_row(row))?;
        Ok(user)
    }
    
    pub fn find_by_username(pool: &DbPool, username: &str) -> DbResult<Self> {
        let conn = pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT id, username, password_hash, email, role, created_at, updated_at 
             FROM users WHERE username = ?"
        )?;
        
        let user = stmt.query_row(params![username], |row| Self::from_row(row))?;
        Ok(user)
    }
    
    pub fn create(pool: &DbPool, new_user: NewUser, password_hash: String) -> DbResult<i64> {
        let conn = pool.get()?;
        let role_str = new_user.role.to_string();
        
        let result = conn.execute(
            "INSERT INTO users (username, password_hash, email, role) 
             VALUES (?, ?, ?, ?)",
            params![
                new_user.username,
                password_hash,
                new_user.email,
                role_str
            ],
        )?;
        
        if result > 0 {
            Ok(conn.last_insert_rowid())
        } else {
            Err(DbError::NoRowsAffected)
        }
    }
    
    pub fn update(pool: &DbPool, id: i64, update: UpdateUser) -> DbResult<()> {
        let conn = pool.get()?;
        let mut query_parts = Vec::new();
        let mut params = Vec::new();
        
        if let Some(username) = update.username {
            query_parts.push("username = ?");
            params.push(username);
        }
        
        if let Some(email) = update.email {
            query_parts.push("email = ?");
            params.push(email);
        }
        
        if let Some(role) = update.role {
            query_parts.push("role = ?");
            params.push(role.to_string());
        }
        
        if query_parts.is_empty() {
            return Ok(());
        }
        
        query_parts.push("updated_at = CURRENT_TIMESTAMP");
        
        let query = format!(
            "UPDATE users SET {} WHERE id = ?",
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
        let result = conn.execute("DELETE FROM users WHERE id = ?", params![id])?;
        
        if result > 0 {
            Ok(())
        } else {
            Err(DbError::NoRowsAffected)
        }
    }
    
    pub fn list(pool: &DbPool) -> DbResult<Vec<Self>> {
        let conn = pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT id, username, password_hash, email, role, created_at, updated_at 
             FROM users ORDER BY username"
        )?;
        
        let users_iter = stmt.query_map([], |row| Self::from_row(row))?;
        let mut users = Vec::new();
        
        for user_result in users_iter {
            match user_result {
                Ok(user) => users.push(user),
                Err(e) => return Err(DbError::from(e)),
            };
        }
        
        Ok(users)
    }
}