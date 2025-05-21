use dotenv::dotenv;
use std::env;
use std::fs;
use std::path::Path;

fn main() {
    // Load environment variables
    dotenv().ok();
    
    // Get database path from environment
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    
    // Delete the database file if it exists
    let db_path = Path::new(&database_url);
    if db_path.exists() {
        println!("Removing existing database file: {}", database_url);
        match fs::remove_file(db_path) {
            Ok(_) => println!("Database file removed successfully"),
            Err(e) => {
                eprintln!("Failed to remove database file: {}", e);
                return;
            }
        }
    }
    
    println!("Database reset complete. Run init_db to recreate the database.");
}