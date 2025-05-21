use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::Error as SqliteError;
use std::path::Path;
use thiserror::Error;

pub mod schema;

#[derive(Debug, Error)]
pub enum DbError {
    #[error("Database error: {0}")]
    Sqlite(#[from] SqliteError),
    #[error("Connection pool error: {0}")]
    Pool(#[from] r2d2::Error),
    #[error("No rows affected")]
    NoRowsAffected,
    #[error("Entity not found")]
    NotFound,
}

pub type DbResult<T> = Result<T, DbError>;
pub type DbPool = Pool<SqliteConnectionManager>;

pub fn init_pool(db_path: &str) -> DbResult<DbPool> {
    let db_path = Path::new(db_path);
    let manager = SqliteConnectionManager::file(db_path);
    let pool = Pool::new(manager)?;
    
    // Initialize the database with the schema
    let conn = pool.get()?;
    schema::initialize_database(&conn)?;
    
    Ok(pool)
}