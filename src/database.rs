use crate::config::CONFIG;
use once_cell::sync::Lazy;
use rusqlite::{Connection, Result, params};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use uuid::Uuid;

pub static DB: Lazy<Mutex<UserDB>> = Lazy::new(|| {
    let db = UserDB::new(CONFIG.database.path.clone()).expect("Failed to open DB");
    Mutex::new(db)
});

#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub username: String,
    pub email: String,
    pub password: String,
    pub verified: bool,
}

pub struct UserDB {
    conn: Connection,
}

impl UserDB {
    /// Create or open database file
    pub fn new(path: String) -> Result<Self> {
        let conn = Connection::open(path)?;
        let db = Self { conn };
        db.create_table()?; // auto-create table if not exists
        Ok(db)
    }

    /// Create users table if it doesn’t exist
    fn create_table(&self) -> Result<()> {
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS users (
                id          TEXT PRIMARY KEY UNIQUE,
                username    TEXT NOT NULL UNIQUE,
                email       TEXT NOT NULL UNIQUE,
                password    TEXT NOT NULL,
                verified    INTEGER NOT NULL DEFAULT 0
            )",
            (),
        )?;
        Ok(())
    }

    pub fn create_user(&self, username: String, email: String, password: String) -> Result<String> {
        let id = Uuid::new_v4().to_string();
        self.conn.execute(
            "INSERT INTO users (id, username, email, password) VALUES (?1, ?2, ?3, ?4)",
            params![id, username, email, password],
        )?;
        Ok(id)
    }

    /// Get user by ID
    pub fn get_user_by_id(&self, id: String) -> Result<Option<User>> {
        self.query_single("SELECT * FROM users WHERE id = ?1", params![id])
    }
    /// Get user by username
    pub fn get_user_by_username(&self, username: &str) -> Result<Option<User>> {
        self.query_single("SELECT * FROM users WHERE username = ?1", params![username])
    }

    /// Get user by email
    pub fn get_user_by_email(&self, email: &str) -> Result<Option<User>> {
        self.query_single("SELECT * FROM users WHERE email = ?1", params![email])
    }

    /// Mark user as verified
    pub fn set_user_verified(&self, id: String) -> Result<()> {
        self.conn
            .execute("UPDATE users SET verified = 1 WHERE id = ?1", params![id])?;
        Ok(())
    }

    /// Internal helper to read single user
    fn query_single(&self, sql: &str, params: impl rusqlite::Params) -> Result<Option<User>> {
        let mut stmt = self.conn.prepare(sql)?;
        let mut rows = stmt.query(params)?;
        if let Some(row) = rows.next()? {
            Ok(Some(User {
                id: row.get(0)?,
                username: row.get(1)?,
                email: row.get(2)?,
                password: row.get(3)?,
                verified: row.get::<_, i64>(4)? != 0,
            }))
        } else {
            Ok(None)
        }
    }
}
