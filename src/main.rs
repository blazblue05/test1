use actix_web::{web, App, HttpServer, middleware::Logger};
use actix_session::{storage::CookieSessionStore, SessionMiddleware};
use actix_web::cookie::Key;
use dotenv::dotenv;
use log::info;
use std::io;
use actix_web::{error, HttpResponse};

mod auth;
mod config;
mod db;
mod handlers;
mod models;
mod utils;

use crate::config::Config;
use crate::db::init_pool;
use crate::handlers::{
    auth as auth_handlers,
    users as user_handlers,
    categories as category_handlers,
    inventory as inventory_handlers,
    transactions as transaction_handlers,
    reports as report_handlers,
};
use crate::models::user::UserRole;
use crate::utils::middleware::{Authentication, RoleAuthorization};

#[actix_web::main]
async fn main() -> io::Result<()> {
    // Initialize environment
    dotenv().ok();
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));
    
    // Load configuration
    let config = Config::from_env();
    let server_host = config.server_host.clone();
    let server_port = config.server_port;
    
    // Initialize database connection pool
    let pool = match init_pool(&config.database_url) {
        Ok(pool) => {
            info!("Database connection pool initialized successfully");
            pool
        },
        Err(e) => {
            eprintln!("Failed to initialize database: {}", e);
            return Err(io::Error::new(io::ErrorKind::Other, "Database initialization failed"));
        }
    };
    
    // Generate a random key for session encryption
    let secret_key = Key::generate();
    
    info!("Starting server at {}:{}", server_host, server_port);
    
    // Start HTTP server
    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .wrap(SessionMiddleware::new(
                CookieSessionStore::default(),
                secret_key.clone(),
            ))
            // Add global JSON extractor config to improve error reporting for 400 errors
            .app_data(
                web::JsonConfig::default()
                    .limit(4096) // 4KB limit, adjust as needed
                    .error_handler(|err, _req| {
                        let msg = format!("Invalid JSON: {}", err);
                        error::InternalError::from_response(
                            err,
                            HttpResponse::BadRequest().json(serde_json::json!({ "error": msg })),
                        )
                        .into()
                    }),
            )
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(config.clone()))
            .service(
                web::scope("/api")
                    // Auth routes (no authentication required)
                    .service(
                        web::scope("/auth")
                            .route("/login", web::post().to(auth_handlers::login))
                    )
                    // User routes (admin only)
                    .service(
                        web::scope("/users")
                            .wrap(Authentication::new(config.jwt_secret.clone()))
                            .wrap(RoleAuthorization::new(vec![UserRole::Admin]))
                            .route("", web::post().to(user_handlers::create_user))
                            .route("", web::get().to(user_handlers::list_users))
                            .route("/{id}", web::get().to(user_handlers::get_user))
                            .route("/{id}", web::put().to(user_handlers::update_user))
                            .route("/{id}", web::delete().to(user_handlers::delete_user))
                    )
                    // Category routes (authenticated)
                    .service(
                        web::scope("/categories")
                            .wrap(Authentication::new(config.jwt_secret.clone()))
                            .route("", web::post().to(category_handlers::create_category))
                            .route("", web::get().to(category_handlers::list_categories))
                            .route("/search", web::get().to(category_handlers::search_categories))
                            .route("/{id}", web::get().to(category_handlers::get_category))
                            .route("/{id}", web::put().to(category_handlers::update_category))
                            .route("/{id}", web::delete().to(category_handlers::delete_category))
                    )
                    // Inventory routes (authenticated)
                    .service(
                        web::scope("/inventory")
                            .wrap(Authentication::new(config.jwt_secret.clone()))
                            .route("", web::post().to(inventory_handlers::create_item))
                            .route("", web::get().to(inventory_handlers::list_items))
                            .route("/search", web::get().to(inventory_handlers::search_items))
                            .route("/low-stock", web::get().to(inventory_handlers::get_low_stock_items))
                            .route("/{id}", web::get().to(inventory_handlers::get_item))
                            .route("/{id}", web::put().to(inventory_handlers::update_item))
                            .route("/{id}", web::delete().to(inventory_handlers::delete_item))
                    )
                    // Transaction routes (authenticated)
                    .service(
                        web::scope("/transactions")
                            .wrap(Authentication::new(config.jwt_secret.clone()))
                            .route("", web::post().to(transaction_handlers::create_transaction))
                            .route("/recent", web::get().to(transaction_handlers::list_recent_transactions))
                            .route("/{id}", web::get().to(transaction_handlers::get_transaction))
                            .route("/item/{id}", web::get().to(transaction_handlers::list_item_transactions))
                            .route("/user/{id}", web::get().to(transaction_handlers::list_user_transactions))
                    )
                    // Report routes (authenticated)
                    .service(
                        web::scope("/reports")
                            .wrap(Authentication::new(config.jwt_secret.clone()))
                            .route("/inventory-summary", web::get().to(report_handlers::get_inventory_summary))
                            .route("/category-summary", web::get().to(report_handlers::get_category_summary))
                            .route("/transaction-history", web::get().to(report_handlers::get_transaction_history))
                    )
            )
    })
    .bind(format!("{}:{}", server_host, server_port))?
    .run()
    .await
}
