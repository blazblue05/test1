use dotenv::dotenv;
use inventory_manager::{
    config::Config,
    db::init_pool,
    models::user::User,
};

fn main() {
    // Load environment variables
    dotenv().ok();
    
    // Load configuration
    let config = Config::from_env();
    
    // Initialize database connection pool
    let pool = match init_pool(&config.database_url) {
        Ok(pool) => {
            println!("Database connection pool initialized successfully");
            pool
        },
        Err(e) => {
            eprintln!("Failed to initialize database: {}", e);
            return;
        }
    };
    
    // List all users
    match User::list(&pool) {
        Ok(users) => {
            println!("Found {} users:", users.len());
            for user in users {
                println!("ID: {:?}, Username: {}, Email: {}, Role: {:?}", 
                    user.id, user.username, user.email, user.role);
            }
        },
        Err(e) => {
            eprintln!("Failed to list users: {}", e);
        }
    }
}