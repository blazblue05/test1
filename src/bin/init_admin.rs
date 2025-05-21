use argon2::{
    password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
    Argon2,
};
use dotenv::dotenv;
use rusqlite::{params, Connection};
use std::env;
use std::fmt;
use std::error::Error;

#[derive(Debug)]
struct AppError(String);

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Error for AppError {}

impl From<rusqlite::Error> for AppError {
    fn from(err: rusqlite::Error) -> Self {
        AppError(err.to_string())
    }
}

impl From<argon2::password_hash::Error> for AppError {
    fn from(err: argon2::password_hash::Error) -> Self {
        AppError(err.to_string())
    }
}

fn main() -> Result<(), AppError> {
    // Load environment variables
    dotenv().ok();
    
    // Get database path from environment
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    
    // Connect to the database
    let conn = Connection::open(&database_url)?;
    
    // Check if admin user already exists
    let admin_exists: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM users WHERE username = 'admin')",
        [],
        |row| row.get(0),
    )?;
    
    if admin_exists {
        println!("Admin user already exists.");
        return Ok(());
    }
    
    // Create admin user
    let username = "admin";
    let password = "admin123"; // Default password
    let email = "admin@example.com";
    let role = "admin";
    
    // Hash the password
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)?
        .to_string();
    
    // Insert admin user
    conn.execute(
        "INSERT INTO users (username, password_hash, email, role, created_at, updated_at) 
         VALUES (?, ?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
        params![username, password_hash, email, role],
    )?;
    
    println!("Admin user created successfully.");
    println!("Username: {}", username);
    println!("Password: {}", password);
    
    Ok(())
}