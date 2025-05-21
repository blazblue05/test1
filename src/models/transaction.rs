use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use rusqlite::{params, Result as SqliteResult, Row};
use crate::db::{DbError, DbPool, DbResult};
use crate::models::inventory_item::InventoryItem;
use crate::models::user::User;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum TransactionType {
    Addition,
    Removal,
    Adjustment,
}

impl TransactionType {
    pub fn from_str(transaction_type: &str) -> Option<Self> {
        match transaction_type.to_lowercase().as_str() {
            "addition" => Some(TransactionType::Addition),
            "removal" => Some(TransactionType::Removal),
            "adjustment" => Some(TransactionType::Adjustment),
            _ => None,
        }
    }
    
    pub fn to_string(&self) -> String {
        match self {
            TransactionType::Addition => "addition".to_string(),
            TransactionType::Removal => "removal".to_string(),
            TransactionType::Adjustment => "adjustment".to_string(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Transaction {
    pub id: Option<i64>,
    pub item_id: i64,
    pub transaction_type: TransactionType,
    pub quantity: i32,
    pub user_id: i64,
    pub notes: Option<String>,
    pub transaction_date: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub item: Option<InventoryItem>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<User>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NewTransaction {
    pub item_id: i64,
    pub transaction_type: TransactionType,
    pub quantity: i32,
    pub user_id: i64,
    pub notes: Option<String>,
}

impl Transaction {
    pub fn from_row(row: &Row) -> SqliteResult<Self> {
        let transaction_type_str: String = row.get("transaction_type")?;
        let transaction_type = TransactionType::from_str(&transaction_type_str)
            .unwrap_or(TransactionType::Adjustment);
        
        let transaction_date_str: String = row.get("transaction_date")?;
        let transaction_date = DateTime::parse_from_rfc3339(&transaction_date_str)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());
        
        Ok(Transaction {
            id: row.get("id")?,
            item_id: row.get("item_id")?,
            transaction_type,
            quantity: row.get("quantity")?,
            user_id: row.get("user_id")?,
            notes: row.get("notes")?,
            transaction_date,
            item: None,
            user: None,
        })
    }
    
    pub fn find_by_id(pool: &DbPool, id: i64, with_relations: bool) -> DbResult<Self> {
        let conn = pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT id, item_id, transaction_type, quantity, user_id, notes, transaction_date 
             FROM inventory_transactions WHERE id = ?"
        )?;
        
        let mut transaction = stmt.query_row(params![id], |row| Self::from_row(row))?;
        
        if with_relations {
            transaction.item = Some(InventoryItem::find_by_id(pool, transaction.item_id, false)?);
            transaction.user = Some(User::find_by_id(pool, transaction.user_id)?);
        }
        
        Ok(transaction)
    }
    
    pub fn create(pool: &DbPool, new_transaction: NewTransaction) -> DbResult<i64> {
        let mut conn = pool.get()?;
        let transaction_type_str = new_transaction.transaction_type.to_string();
        
        // Start a transaction to ensure atomicity
        let tx = conn.transaction()?;
        
        // Insert the transaction record
        let result = tx.execute(
            "INSERT INTO inventory_transactions (item_id, transaction_type, quantity, user_id, notes) 
             VALUES (?, ?, ?, ?, ?)",
            params![
                new_transaction.item_id,
                transaction_type_str,
                new_transaction.quantity,
                new_transaction.user_id,
                new_transaction.notes,
            ],
        )?;
        
        if result == 0 {
            return Err(DbError::NoRowsAffected);
        }
        
        let transaction_id = tx.last_insert_rowid();
        
        // Get current quantity
        let current_quantity: i32 = {
            let mut stmt = tx.prepare("SELECT quantity FROM inventory_items WHERE id = ?")?;
            stmt.query_row(params![new_transaction.item_id], |row| row.get(0))?
        };
        
        // Calculate new quantity
        let new_quantity = match new_transaction.transaction_type {
            TransactionType::Addition => current_quantity + new_transaction.quantity,
            TransactionType::Removal => current_quantity - new_transaction.quantity,
            TransactionType::Adjustment => new_transaction.quantity,
        };
        
        // Update quantity
        let update_result = tx.execute(
            "UPDATE inventory_items SET quantity = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
            params![new_quantity, new_transaction.item_id],
        )?;
        
        if update_result == 0 {
            return Err(DbError::NoRowsAffected);
        }
        
        // Commit the transaction
        tx.commit()?;
        
        Ok(transaction_id)
    }
    
    pub fn list_by_item(pool: &DbPool, item_id: i64, with_relations: bool) -> DbResult<Vec<Self>> {
        let conn = pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT id, item_id, transaction_type, quantity, user_id, notes, transaction_date 
             FROM inventory_transactions 
             WHERE item_id = ? 
             ORDER BY transaction_date DESC"
        )?;
        
        let transactions_iter = stmt.query_map(params![item_id], |row| Self::from_row(row))?;
        let mut transactions = Vec::new();
        
        for transaction_result in transactions_iter {
            match transaction_result {
                Ok(mut transaction) => {
                    if with_relations {
                        transaction.item = Some(InventoryItem::find_by_id(pool, transaction.item_id, false)?);
                        transaction.user = Some(User::find_by_id(pool, transaction.user_id)?);
                    }
                    
                    transactions.push(transaction);
                },
                Err(e) => return Err(DbError::from(e)),
            }
        }
        
        Ok(transactions)
    }
    
    pub fn list_by_user(pool: &DbPool, user_id: i64, with_relations: bool) -> DbResult<Vec<Self>> {
        let conn = pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT id, item_id, transaction_type, quantity, user_id, notes, transaction_date 
             FROM inventory_transactions 
             WHERE user_id = ? 
             ORDER BY transaction_date DESC"
        )?;
        
        let transactions_iter = stmt.query_map(params![user_id], |row| Self::from_row(row))?;
        let mut transactions = Vec::new();
        
        for transaction_result in transactions_iter {
            match transaction_result {
                Ok(mut transaction) => {
                    if with_relations {
                        transaction.item = Some(InventoryItem::find_by_id(pool, transaction.item_id, false)?);
                        transaction.user = Some(User::find_by_id(pool, transaction.user_id)?);
                    }
                    
                    transactions.push(transaction);
                },
                Err(e) => return Err(DbError::from(e)),
            }
        }
        
        Ok(transactions)
    }
    
    pub fn list_recent(pool: &DbPool, limit: i64, with_relations: bool) -> DbResult<Vec<Self>> {
        let conn = pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT id, item_id, transaction_type, quantity, user_id, notes, transaction_date 
             FROM inventory_transactions 
             ORDER BY transaction_date DESC 
             LIMIT ?"
        )?;
        
        let transactions_iter = stmt.query_map(params![limit], |row| Self::from_row(row))?;
        let mut transactions = Vec::new();
        
        for transaction_result in transactions_iter {
            match transaction_result {
                Ok(mut transaction) => {
                    if with_relations {
                        transaction.item = Some(InventoryItem::find_by_id(pool, transaction.item_id, false)?);
                        transaction.user = Some(User::find_by_id(pool, transaction.user_id)?);
                    }
                    
                    transactions.push(transaction);
                },
                Err(e) => return Err(DbError::from(e)),
            }
        }
        
        Ok(transactions)
    }
}