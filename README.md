# Inventory Manager

A comprehensive inventory management system built with Rust, featuring a web API and CLI interface for managing inventory items, categories, transactions, and generating reports.

## Features

- **User Authentication**: Secure login with JWT-based authentication
- **Role-Based Access Control**: Admin and regular user roles with appropriate permissions
- **Inventory Management**: Create, read, update, and delete inventory items
- **Category Management**: Organize items into categories
- **Transaction Tracking**: Record inventory movements (additions, removals)
- **Reporting**: Generate inventory summaries and category-based reports
- **Low Stock Alerts**: Identify items with low stock levels
- **Web API**: RESTful API for integration with other systems
- **CLI Interface**: Command-line interface for quick access to common functions

## Technology Stack

- **Backend**: Rust with Actix-web framework
- **Database**: SQLite with Rusqlite and R2D2 connection pooling
- **Authentication**: JWT (JSON Web Tokens)
- **Password Security**: Argon2 password hashing
- **CLI**: Clap for command parsing, Reqwest for API communication
- **Serialization**: Serde for JSON handling

## Getting Started

### Prerequisites

- Rust and Cargo (latest stable version)
- SQLite

### Installation

1. Clone the repository:
   ```
   git clone https://github.com/yourusername/inventory_manager.git
   cd inventory_manager
   ```

2. Configure the environment:
   Create a `.env` file in the project root with the following content:
   ```
   DATABASE_URL=inventory.db
   JWT_SECRET=your_secure_jwt_secret_key_change_this_in_production
   JWT_EXPIRATION=86400
   SERVER_HOST=127.0.0.1
   SERVER_PORT=8080
   ```

3. Initialize the database:
   ```
   cargo run --bin init_db
   ```
   This will create the database schema and add an admin user with the following credentials:
   - Username: admin
   - Password: admin123

   **Note**: Change the admin password after first login for security.

4. Start the application:
   ```
   cargo run
   ```
   Or use the provided batch script (Windows):
   ```
   start_inventory_system.bat
   ```

### Using the CLI

The CLI provides an interactive interface to the inventory system:

1. Start the CLI:
   ```
   cargo run --bin inventory_cli
   ```

2. Login with your credentials

3. Use the interactive menu to:
   - View inventory dashboard
   - Manage inventory items
   - Manage categories
   - Record transactions
   - Generate reports

## API Endpoints

### Authentication
- `POST /api/auth/login` - Authenticate user and get JWT token

### Users (Admin only)
- `POST /api/users` - Create a new user
- `GET /api/users` - List all users
- `GET /api/users/{id}` - Get user details
- `PUT /api/users/{id}` - Update user
- `DELETE /api/users/{id}` - Delete user

### Categories
- `POST /api/categories` - Create a new category
- `GET /api/categories` - List all categories
- `GET /api/categories/search` - Search categories
- `GET /api/categories/{id}` - Get category details
- `PUT /api/categories/{id}` - Update category
- `DELETE /api/categories/{id}` - Delete category

### Inventory
- `POST /api/inventory` - Create a new inventory item
- `GET /api/inventory` - List all inventory items
- `GET /api/inventory/search` - Search inventory items
- `GET /api/inventory/low-stock` - Get low stock items
- `GET /api/inventory/{id}` - Get item details
- `PUT /api/inventory/{id}` - Update item
- `DELETE /api/inventory/{id}` - Delete item

### Transactions
- `POST /api/transactions` - Create a new transaction
- `GET /api/transactions/recent` - List recent transactions
- `GET /api/transactions/{id}` - Get transaction details
- `GET /api/transactions/item/{id}` - List transactions for an item
- `GET /api/transactions/user/{id}` - List transactions by a user

### Reports
- `GET /api/reports/inventory-summary` - Get inventory summary
- `GET /api/reports/category-summary` - Get category summary
- `GET /api/reports/transaction-history` - Get transaction history

## Development

### Project Structure

- `src/`
  - `auth/` - Authentication and password handling
  - `bin/` - Binary executables (CLI, database initialization)
  - `config/` - Application configuration
  - `db/` - Database connection and schema
  - `handlers/` - API request handlers
  - `models/` - Data models
  - `utils/` - Utility functions and middleware
  - `main.rs` - Web server entry point
  - `lib.rs` - Shared library code

### Building

```
cargo build --release
```

### Testing

```
cargo test
```

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Acknowledgments

- The Rust community for excellent libraries and documentation
- Contributors to the project