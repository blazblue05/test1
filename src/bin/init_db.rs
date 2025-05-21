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
    
    // Create admin user
    let admin_password = "admin123"; // Change this to a secure password
    let password_hash = match password::hash_password(admin_password) {
        Ok(hash) => hash,
        Err(e) => {
            eprintln!("Failed to hash password: {}", e);
            return;
        }
    };
    
    let new_admin = NewUser {
        username: "admin".to_string(),
        password: admin_password.to_string(),
        email: "admin@example.com".to_string(),
        role: UserRole::Admin,
    };
    
    match User::create(&pool, new_admin, password_hash) {
        Ok(user_id) => {
            println!("Admin user created with ID: {}", user_id);
        },
        Err(e) => {
            if e.to_string().contains("UNIQUE constraint failed") {
                println!("Admin user already exists");
            } else {
                eprintln!("Failed to create admin user: {}", e);
            }
        }
    }
    
    // Create some sample categories
    use inventory_manager::models::category::{Category, NewCategory};
    
    let categories = vec![
        NewCategory {
            name: "Electronics".to_string(),
            description: Some("Electronic devices and components".to_string()),
        },
        NewCategory {
            name: "Office Supplies".to_string(),
            description: Some("Supplies for office use".to_string()),
        },
        NewCategory {
            name: "Furniture".to_string(),
            description: Some("Office and home furniture".to_string()),
        },
    ];
    
    for category in categories {
        match Category::create(&pool, category.clone()) {
            Ok(category_id) => {
                println!("Category '{}' created with ID: {}", category.name, category_id);
            },
            Err(e) => {
                if e.to_string().contains("UNIQUE constraint failed") {
                    println!("Category '{}' already exists", category.name);
                } else {
                    eprintln!("Failed to create category '{}': {}", category.name, e);
                }
            }
        }
    }
    
    println!("Database initialization completed");
}