use dotenv::dotenv;
use inventory_manager::{
    auth::password,
    config::Config,
    db::init_pool,
    models::user::{NewUser, User, UserRole},
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
    
    // Create test admin user
    let username = "testadmin";
    let admin_password = "password123";
    let password_hash = match password::hash_password(admin_password) {
        Ok(hash) => hash,
        Err(e) => {
            eprintln!("Failed to hash password: {}", e);
            return;
        }
    };
    
    let new_admin = NewUser {
        username: username.to_string(),
        password: admin_password.to_string(),
        email: "testadmin@example.com".to_string(),
        role: UserRole::Admin,
    };
    
    match User::create(&pool, new_admin, password_hash) {
        Ok(user_id) => {
            println!("Test admin user created with ID: {}", user_id);
            println!("Username: {}", username);
            println!("Password: {}", admin_password);
        },
        Err(e) => {
            if e.to_string().contains("UNIQUE constraint failed") {
                println!("Test admin user already exists");
            } else {
                eprintln!("Failed to create test admin user: {}", e);
            }
        }
    }
}