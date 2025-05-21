use clap::{App, AppSettings, Arg, SubCommand};
use reqwest::{Client, header};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::{self, Write};
use std::error::Error;
use std::fmt;
use tokio;
use colored::*;
use prettytable::{Table, Row, Cell, format};
use dialoguer::{Input, Password, Select, Confirm};

#[derive(Debug, Serialize, Deserialize)]
struct LoginRequest {
    username: String,
    password: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct LoginResponse {
    token: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ErrorResponse {
    error: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct InventorySummary {
    total_items: i64,
    total_quantity: i64,
    total_value: f64,
    categories_count: i64,
    low_stock_count: i64,
    zero_stock_count: i64,
}

#[derive(Debug, Serialize, Deserialize)]
struct CategorySummary {
    id: i64,
    name: String,
    items_count: i64,
    total_quantity: i64,
    total_value: f64,
}

#[derive(Debug, Serialize, Deserialize)]
struct Category {
    id: Option<i64>,
    name: String,
    description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct InventoryItem {
    id: Option<i64>,
    name: String,
    description: Option<String>,
    category_id: i64,
    quantity: i32,
    unit_price: f64,
    sku: Option<String>,
    location: Option<String>,
    category: Option<Category>,
}

#[derive(Debug, Serialize, Deserialize)]
struct NewInventoryItem {
    name: String,
    description: Option<String>,
    category_id: i64,
    quantity: i32,
    unit_price: f64,
    sku: Option<String>,
    location: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Transaction {
    id: Option<i64>,
    item_id: i64,
    user_id: i64,
    quantity: i32,
    transaction_type: String,
    notes: Option<String>,
    transaction_date: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct NewTransaction {
    item_id: i64,
    quantity: i32,
    transaction_type: String,
    notes: Option<String>,
}

struct CliError(String);

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl fmt::Debug for CliError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "CliError: {}", self.0)
    }
}

impl Error for CliError {}

impl From<std::io::Error> for CliError {
    fn from(err: std::io::Error) -> Self {
        CliError(format!("IO error: {}", err))
    }
}

type CliResult<T> = Result<T, Box<dyn Error + 'static>>;

struct InventoryCli {
    client: Client,
    base_url: String,
    token: Option<String>,
}

impl InventoryCli {
    fn new(base_url: &str) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.to_string(),
            token: None,
        }
    }

    async fn login(&mut self, username: &str, password: &str) -> CliResult<()> {
        let login_request = LoginRequest {
            username: username.to_string(),
            password: password.to_string(),
        };

        // Add a timeout to the request to prevent hanging
        let response = match self.client
            .post(&format!("{}/api/auth/login", self.base_url))
            .json(&login_request)
            .timeout(std::time::Duration::from_secs(5))
            .send()
            .await {
                Ok(resp) => resp,
                Err(e) => {
                    // Handle connection errors more gracefully
                    if e.is_timeout() {
                        return Err(Box::new(CliError("Connection timed out. Is the server running?".to_string())));
                    } else if e.is_connect() {
                        return Err(Box::new(CliError(format!("Failed to connect to server at {}. Is the server running?", self.base_url))));
                    } else {
                        return Err(Box::new(CliError(format!("Network error: {}", e))));
                    }
                }
            };

        let status = response.status();
        if status.is_success() {
            // Try to parse the response, handle parsing errors gracefully
            match response.json::<LoginResponse>().await {
                Ok(login_response) => {
                    self.token = Some(login_response.token);
                    Ok(())
                },
                Err(e) => {
                    Err(Box::new(CliError(format!("Failed to parse login response: {}", e))))
                }
            }
        } else {
            // Try to parse error response, fall back to status code if parsing fails
            let error_text = match response.json::<ErrorResponse>().await {
                Ok(error) => error.error,
                Err(_) => format!("Login failed with status: {}", status)
            };
            Err(Box::new(CliError(error_text)))
        }
    }

    fn get_auth_header(&self) -> CliResult<header::HeaderMap> {
        let mut headers = header::HeaderMap::new();
        if let Some(token) = &self.token {
            headers.insert(
                header::AUTHORIZATION,
                header::HeaderValue::from_str(&format!("Bearer {}", token))?,
            );
            Ok(headers)
        } else {
            Err(Box::new(CliError("Not logged in".to_string())))
        }
    }

    async fn get_inventory_summary(&self) -> CliResult<InventorySummary> {
        let headers = self.get_auth_header()?;
        let response = self.client
            .get(&format!("{}/api/reports/inventory-summary", self.base_url))
            .headers(headers)
            .send()
            .await?;

        if response.status().is_success() {
            let summary: InventorySummary = response.json().await?;
            Ok(summary)
        } else {
            let error: ErrorResponse = response.json().await?;
            Err(Box::new(CliError(error.error)))
        }
    }

    async fn get_category_summary(&self) -> CliResult<Vec<CategorySummary>> {
        let headers = self.get_auth_header()?;
        let response = self.client
            .get(&format!("{}/api/reports/category-summary", self.base_url))
            .headers(headers)
            .send()
            .await?;

        if response.status().is_success() {
            let summary: Vec<CategorySummary> = response.json().await?;
            Ok(summary)
        } else {
            let error: ErrorResponse = response.json().await?;
            Err(Box::new(CliError(error.error)))
        }
    }

    async fn list_categories(&self) -> CliResult<Vec<Category>> {
        let headers = self.get_auth_header()?;
        let response = self.client
            .get(&format!("{}/api/categories", self.base_url))
            .headers(headers)
            .send()
            .await?;

        if response.status().is_success() {
            let categories: Vec<Category> = response.json().await?;
            Ok(categories)
        } else {
            let error: ErrorResponse = response.json().await?;
            Err(Box::new(CliError(error.error)))
        }
    }

    async fn create_category(&self, name: &str, description: Option<&str>) -> CliResult<Category> {
        let headers = self.get_auth_header()?;
        let category = Category {
            id: None,
            name: name.to_string(),
            description: description.map(|s| s.to_string()),
        };

        let response = self.client
            .post(&format!("{}/api/categories", self.base_url))
            .headers(headers)
            .json(&category)
            .send()
            .await?;

        if response.status().is_success() {
            let created_category: Category = response.json().await?;
            Ok(created_category)
        } else {
            let error: ErrorResponse = response.json().await?;
            Err(Box::new(CliError(error.error)))
        }
    }

    async fn list_inventory(&self) -> CliResult<Vec<InventoryItem>> {
        let headers = self.get_auth_header()?;
        let response = self.client
            .get(&format!("{}/api/inventory", self.base_url))
            .headers(headers)
            .send()
            .await?;

        if response.status().is_success() {
            let items: Vec<InventoryItem> = response.json().await?;
            Ok(items)
        } else {
            let error: ErrorResponse = response.json().await?;
            Err(Box::new(CliError(error.error)))
        }
    }

    async fn get_inventory_item(&self, id: i64) -> CliResult<InventoryItem> {
        let headers = self.get_auth_header()?;
        let response = self.client
            .get(&format!("{}/api/inventory/{}", self.base_url, id))
            .headers(headers)
            .send()
            .await?;

        if response.status().is_success() {
            let item: InventoryItem = response.json().await?;
            Ok(item)
        } else {
            let error: ErrorResponse = response.json().await?;
            Err(Box::new(CliError(error.error)))
        }
    }

    async fn create_inventory_item(&self, item: NewInventoryItem) -> CliResult<InventoryItem> {
        let headers = self.get_auth_header()?;
        let response = self.client
            .post(&format!("{}/api/inventory", self.base_url))
            .headers(headers)
            .json(&item)
            .send()
            .await?;

        if response.status().is_success() {
            let created_item: InventoryItem = response.json().await?;
            Ok(created_item)
        } else {
            let error: ErrorResponse = response.json().await?;
            Err(Box::new(CliError(error.error)))
        }
    }

    async fn create_transaction(&self, transaction: NewTransaction) -> CliResult<Transaction> {
        let headers = self.get_auth_header()?;
        let response = self.client
            .post(&format!("{}/api/transactions", self.base_url))
            .headers(headers)
            .json(&transaction)
            .send()
            .await?;

        if response.status().is_success() {
            let created_transaction: Transaction = response.json().await?;
            Ok(created_transaction)
        } else {
            let error: ErrorResponse = response.json().await?;
            Err(Box::new(CliError(error.error)))
        }
    }

    async fn list_recent_transactions(&self) -> CliResult<Vec<Transaction>> {
        let headers = self.get_auth_header()?;
        let response = self.client
            .get(&format!("{}/api/transactions/recent", self.base_url))
            .headers(headers)
            .send()
            .await?;

        if response.status().is_success() {
            let transactions: Vec<Transaction> = response.json().await?;
            Ok(transactions)
        } else {
            let error: ErrorResponse = response.json().await?;
            Err(Box::new(CliError(error.error)))
        }
    }
}

async fn interactive_login(cli: &mut InventoryCli) -> CliResult<()> {
    println!("{}", "Login to Inventory Management System".green().bold());
    println!("Please enter your credentials to continue.");
    
    let username = match Input::<String>::new()
        .with_prompt("Username")
        .interact() {
            Ok(username) => username,
            Err(e) => return Err(Box::new(CliError(format!("Failed to read username: {}", e))))
        };
    
    let password = match Password::new()
        .with_prompt("Password")
        .interact() {
            Ok(password) => password,
            Err(e) => return Err(Box::new(CliError(format!("Failed to read password: {}", e))))
        };
    
    println!("Authenticating... Please wait");
    
    match cli.login(&username, &password).await {
        Ok(_) => {
            println!("{}", "✓ Login successful!".green().bold());
            Ok(())
        },
        Err(e) => {
            // Error is already printed in the main function
            Err(e)
        }
    }
}

async fn display_dashboard(cli: &InventoryCli) -> CliResult<()> {
    println!("\n{}", "=== Inventory Dashboard ===".green().bold());
    
    // Get inventory summary
    let summary = cli.get_inventory_summary().await?;
    
    let mut table = Table::new();
    table.set_format(*format::consts::FORMAT_NO_BORDER_LINE_SEPARATOR);
    
    table.add_row(Row::new(vec![
        Cell::new("Total Items").style_spec("Fb"),
        Cell::new(&summary.total_items.to_string()),
    ]));
    
    table.add_row(Row::new(vec![
        Cell::new("Total Quantity").style_spec("Fb"),
        Cell::new(&summary.total_quantity.to_string()),
    ]));
    
    table.add_row(Row::new(vec![
        Cell::new("Total Value").style_spec("Fb"),
        Cell::new(&format!("${:.2}", summary.total_value)),
    ]));
    
    table.add_row(Row::new(vec![
        Cell::new("Categories").style_spec("Fb"),
        Cell::new(&summary.categories_count.to_string()),
    ]));
    
    table.add_row(Row::new(vec![
        Cell::new("Low Stock Items").style_spec("Fb"),
        Cell::new(&summary.low_stock_count.to_string()).style_spec("Fr"),
    ]));
    
    table.add_row(Row::new(vec![
        Cell::new("Out of Stock Items").style_spec("Fb"),
        Cell::new(&summary.zero_stock_count.to_string()).style_spec("Fr"),
    ]));
    
    table.printstd();
    
    Ok(())
}

async fn list_categories(cli: &InventoryCli) -> CliResult<()> {
    println!("\n{}", "=== Categories ===".green().bold());
    
    let categories = cli.list_categories().await?;
    
    if categories.is_empty() {
        println!("No categories found.");
        return Ok(());
    }
    
    let mut table = Table::new();
    table.set_format(*format::consts::FORMAT_NO_BORDER_LINE_SEPARATOR);
    
    table.add_row(Row::new(vec![
        Cell::new("ID").style_spec("Fb"),
        Cell::new("Name").style_spec("Fb"),
        Cell::new("Description").style_spec("Fb"),
    ]));
    
    for category in categories {
        table.add_row(Row::new(vec![
            Cell::new(&category.id.unwrap_or(0).to_string()),
            Cell::new(&category.name),
            Cell::new(category.description.as_deref().unwrap_or("-")),
        ]));
    }
    
    table.printstd();
    
    Ok(())
}

async fn list_inventory(cli: &InventoryCli) -> CliResult<()> {
    println!("\n{}", "=== Inventory Items ===".green().bold());
    
    let items = cli.list_inventory().await?;
    
    if items.is_empty() {
        println!("No inventory items found.");
        return Ok(());
    }
    
    let mut table = Table::new();
    table.set_format(*format::consts::FORMAT_NO_BORDER_LINE_SEPARATOR);
    
    table.add_row(Row::new(vec![
        Cell::new("ID").style_spec("Fb"),
        Cell::new("Name").style_spec("Fb"),
        Cell::new("Category").style_spec("Fb"),
        Cell::new("Quantity").style_spec("Fb"),
        Cell::new("Price").style_spec("Fb"),
        Cell::new("Value").style_spec("Fb"),
        Cell::new("Location").style_spec("Fb"),
    ]));
    
    for item in items {
        let category_name = match &item.category {
            Some(cat) => cat.name.clone(),
            None => "-".to_string(),
        };
        
        let quantity_cell = if item.quantity <= 0 {
            Cell::new(&item.quantity.to_string()).style_spec("Fr")
        } else if item.quantity <= 10 {
            Cell::new(&item.quantity.to_string()).style_spec("Fy")
        } else {
            Cell::new(&item.quantity.to_string())
        };
        
        table.add_row(Row::new(vec![
            Cell::new(&item.id.unwrap_or(0).to_string()),
            Cell::new(&item.name),
            Cell::new(&category_name),
            quantity_cell,
            Cell::new(&format!("${:.2}", item.unit_price)),
            Cell::new(&format!("${:.2}", item.quantity as f64 * item.unit_price)),
            Cell::new(item.location.as_deref().unwrap_or("-")),
        ]));
    }
    
    table.printstd();
    
    Ok(())
}

async fn add_inventory_item(cli: &InventoryCli) -> CliResult<()> {
    println!("\n{}", "=== Add Inventory Item ===".green().bold());
    
    // First, get categories for selection
    let categories = cli.list_categories().await?;
    
    if categories.is_empty() {
        println!("{}", "No categories available. Please create a category first.".red());
        return Ok(());
    }
    
    // Prepare category selection
    let category_names: Vec<String> = categories.iter()
        .map(|c| c.name.clone())
        .collect();
    
    let name = Input::<String>::new()
        .with_prompt("Item name")
        .interact()?;
    
    let description = Input::<String>::new()
        .with_prompt("Description (optional)")
        .allow_empty(true)
        .interact()?;
    
    let category_index = Select::new()
        .with_prompt("Select category")
        .items(&category_names)
        .default(0)
        .interact()?;
    
    let category_id = categories[category_index].id.unwrap();
    
    let quantity = Input::<i32>::new()
        .with_prompt("Quantity")
        .default(0)
        .interact()?;
    
    let unit_price = Input::<f64>::new()
        .with_prompt("Unit price")
        .default(0.0)
        .interact()?;
    
    let sku = Input::<String>::new()
        .with_prompt("SKU (optional)")
        .allow_empty(true)
        .interact()?;
    
    let location = Input::<String>::new()
        .with_prompt("Location (optional)")
        .allow_empty(true)
        .interact()?;
    
    let new_item = NewInventoryItem {
        name,
        description: if description.is_empty() { None } else { Some(description) },
        category_id,
        quantity,
        unit_price,
        sku: if sku.is_empty() { None } else { Some(sku) },
        location: if location.is_empty() { None } else { Some(location) },
    };
    
    match cli.create_inventory_item(new_item).await {
        Ok(item) => {
            println!("{} {}", "Successfully added item:".green(), item.name);
            Ok(())
        },
        Err(e) => {
            println!("{}: {}", "Failed to add item".red(), e);
            Err(e)
        }
    }
}

async fn add_category(cli: &InventoryCli) -> CliResult<()> {
    println!("\n{}", "=== Add Category ===".green().bold());
    
    let name = Input::<String>::new()
        .with_prompt("Category name")
        .interact()?;
    
    let description = Input::<String>::new()
        .with_prompt("Description (optional)")
        .allow_empty(true)
        .interact()?;
    
    match cli.create_category(&name, if description.is_empty() { None } else { Some(&description) }).await {
        Ok(category) => {
            println!("{} {}", "Successfully added category:".green(), category.name);
            Ok(())
        },
        Err(e) => {
            println!("{}: {}", "Failed to add category".red(), e);
            Err(e)
        }
    }
}

async fn add_transaction(cli: &InventoryCli) -> CliResult<()> {
    println!("\n{}", "=== Add Transaction ===".green().bold());
    
    // First, get inventory items for selection
    let items = cli.list_inventory().await?;
    
    if items.is_empty() {
        println!("{}", "No inventory items available. Please add an item first.".red());
        return Ok(());
    }
    
    // Prepare item selection
    let item_names: Vec<String> = items.iter()
        .map(|i| format!("{} (Qty: {})", i.name, i.quantity))
        .collect();
    
    let item_index = Select::new()
        .with_prompt("Select item")
        .items(&item_names)
        .default(0)
        .interact()?;
    
    let item_id = items[item_index].id.unwrap();
    let item_name = &items[item_index].name;
    let current_quantity = items[item_index].quantity;
    
    println!("Current quantity of {}: {}", item_name, current_quantity);
    
    let transaction_types = vec!["addition", "removal", "adjustment"];
    let transaction_type_index = Select::new()
        .with_prompt("Transaction type")
        .items(&transaction_types)
        .default(0)
        .interact()?;
    
    let transaction_type = transaction_types[transaction_type_index].to_string();
    
    let quantity = Input::<i32>::new()
        .with_prompt("Quantity")
        .default(1)
        .interact()?;
    
    let notes = Input::<String>::new()
        .with_prompt("Notes (optional)")
        .allow_empty(true)
        .interact()?;
    
    let new_transaction = NewTransaction {
        item_id,
        quantity,
        transaction_type,
        notes: if notes.is_empty() { None } else { Some(notes) },
    };
    
    match cli.create_transaction(new_transaction).await {
        Ok(_transaction) => {
            println!("{} for item: {}", "Successfully added transaction".green(), item_name);
            Ok(())
        },
        Err(e) => {
            println!("{}: {}", "Failed to add transaction".red(), e);
            Err(e)
        }
    }
}

async fn list_transactions(cli: &InventoryCli) -> CliResult<()> {
    println!("\n{}", "=== Recent Transactions ===".green().bold());
    
    let transactions = cli.list_recent_transactions().await?;
    
    if transactions.is_empty() {
        println!("No transactions found.");
        return Ok(());
    }
    
    let mut table = Table::new();
    table.set_format(*format::consts::FORMAT_NO_BORDER_LINE_SEPARATOR);
    
    table.add_row(Row::new(vec![
        Cell::new("ID").style_spec("Fb"),
        Cell::new("Date").style_spec("Fb"),
        Cell::new("Item ID").style_spec("Fb"),
        Cell::new("Type").style_spec("Fb"),
        Cell::new("Quantity").style_spec("Fb"),
        Cell::new("Notes").style_spec("Fb"),
    ]));
    
    for transaction in transactions {
        let type_cell = match transaction.transaction_type.as_str() {
            "addition" => Cell::new("Addition").style_spec("Fg"),
            "removal" => Cell::new("Removal").style_spec("Fr"),
            "adjustment" => Cell::new("Adjustment").style_spec("Fb"),
            _ => Cell::new(&transaction.transaction_type),
        };
        
        table.add_row(Row::new(vec![
            Cell::new(&transaction.id.unwrap_or(0).to_string()),
            Cell::new(&transaction.transaction_date),
            Cell::new(&transaction.item_id.to_string()),
            type_cell,
            Cell::new(&transaction.quantity.to_string()),
            Cell::new(transaction.notes.as_deref().unwrap_or("-")),
        ]));
    }
    
    table.printstd();
    
    Ok(())
}

async fn show_category_summary(cli: &InventoryCli) -> CliResult<()> {
    println!("\n{}", "=== Category Summary ===".green().bold());
    
    let summaries = cli.get_category_summary().await?;
    
    if summaries.is_empty() {
        println!("No category data available.");
        return Ok(());
    }
    
    let mut table = Table::new();
    table.set_format(*format::consts::FORMAT_NO_BORDER_LINE_SEPARATOR);
    
    table.add_row(Row::new(vec![
        Cell::new("Category").style_spec("Fb"),
        Cell::new("Items").style_spec("Fb"),
        Cell::new("Quantity").style_spec("Fb"),
        Cell::new("Value").style_spec("Fb"),
    ]));
    
    for summary in summaries {
        table.add_row(Row::new(vec![
            Cell::new(&summary.name),
            Cell::new(&summary.items_count.to_string()),
            Cell::new(&summary.total_quantity.to_string()),
            Cell::new(&format!("${:.2}", summary.total_value)),
        ]));
    }
    
    table.printstd();
    
    Ok(())
}

async fn main_menu(cli: &mut InventoryCli) -> CliResult<bool> {
    let options = vec![
        "Dashboard",
        "List Inventory",
        "Add Inventory Item",
        "List Categories",
        "Add Category",
        "Add Transaction",
        "List Recent Transactions",
        "Category Summary",
        "Exit",
    ];
    
    let selection = Select::new()
        .with_prompt("Select an option")
        .items(&options)
        .default(0)
        .interact()?;
    
    match selection {
        0 => {
            display_dashboard(cli).await?;
        },
        1 => {
            list_inventory(cli).await?;
        },
        2 => {
            add_inventory_item(cli).await?;
        },
        3 => {
            list_categories(cli).await?;
        },
        4 => {
            add_category(cli).await?;
        },
        5 => {
            add_transaction(cli).await?;
        },
        6 => {
            list_transactions(cli).await?;
        },
        7 => {
            show_category_summary(cli).await?;
        },
        8 => {
            println!("Exiting...");
            return Ok(false);
        },
        _ => {}
    }
    
    Ok(true)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Allow configuring the server URL via environment variable
    let server_url = std::env::var("INVENTORY_SERVER_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:8080".to_string());
    
    // Check if the server is running before proceeding
    println!("Checking server connection...");
    match reqwest::get(&format!("{}/api/auth/login", server_url)).await {
        Ok(_) => println!("Server is running at {}", server_url.cyan()),
        Err(e) => {
            if e.is_connect() {
                println!("{}: Server at {} is not running or unreachable.", "ERROR".red().bold(), server_url);
                println!("Please start the server or set the correct URL using the INVENTORY_SERVER_URL environment variable.");
                println!("Example: INVENTORY_SERVER_URL=http://localhost:3000 cargo run --bin inventory_cli");
                return Err(Box::new(CliError(format!("Server connection failed: {}", e))) as Box<dyn Error>);
            } else {
                println!("{}: {}", "WARNING".yellow().bold(), e);
                println!("Continuing anyway, but you may encounter issues...");
            }
        }
    }
    
    let mut cli = InventoryCli::new(&server_url);
    
    println!("{}", "Inventory Management CLI".green().bold());
    println!("{}", "======================".green());
    println!("Connecting to server: {}", server_url.cyan());
    
    // Login with retry option
    let mut login_attempts = 0;
    let max_attempts = 3;
    
    loop {
        match interactive_login(&mut cli).await {
            Ok(_) => break, // Login successful
            Err(e) => {
                login_attempts += 1;
                println!("{}: {}", "Login failed".red().bold(), e);
                
                if login_attempts >= max_attempts {
                    println!("Maximum login attempts reached. Exiting.");
                    return Err(Box::new(CliError("Authentication failed after multiple attempts".to_string())) as Box<dyn Error>);
                }
                
                // Ask if user wants to retry
                let retry = match Confirm::new()
                    .with_prompt("Would you like to try again?")
                    .default(true)
                    .interact() {
                        Ok(result) => result,
                        Err(e) => {
                            println!("Error reading input: {}", e);
                            false
                        }
                    };
                
                if !retry {
                    return Err(Box::new(CliError("Login cancelled by user".to_string())) as Box<dyn Error>);
                }
            }
        }
    }
    
    // Main application loop
    loop {
        match main_menu(&mut cli).await {
            Ok(continue_running) => {
                if !continue_running {
                    break;
                }
            },
            Err(e) => {
                println!("{}: {}", "An error occurred".yellow(), e);
                println!("Returning to main menu.");
            }
        }
        
        println!("\nPress Enter to continue...");
        match io::stdout().flush() {
            Ok(_) => {},
            Err(e) => println!("Error flushing stdout: {}", e),
        }
        
        let mut input = String::new();
        match io::stdin().read_line(&mut input) {
            Ok(_) => {},
            Err(e) => println!("Error reading input: {}", e),
        }
    }
    
    Ok(())
}