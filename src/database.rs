use crate::config::CONFIG;
use bcrypt::{hash, verify};
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
    pub role: String,
    pub verified: bool,
}

pub struct UserDB {
    conn: Connection,
}

impl UserDB {
    pub fn new(path: String) -> Result<Self> {
        let conn = Connection::open(path)?;
        let db = Self { conn };
        db.create_table()?; // auto-create table if not exists
        Ok(db)
    }

    fn create_table(&self) -> Result<()> {
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS users (
                id          TEXT PRIMARY KEY UNIQUE,
                username    TEXT NOT NULL UNIQUE,
                email       TEXT NOT NULL UNIQUE,
                role        TEXT NOT NULL DEFAULT 'user',
                password    TEXT NOT NULL,
                verified    INTEGER NOT NULL DEFAULT 0
            )",
            (),
        )?;
        Ok(())
    }

    pub fn create_user(
        &self,
        username: String,
        email: String,
        password: String,
    ) -> Result<User, String> {
        let id = Uuid::new_v4().to_string();
        let hashed_pass = hash(password, CONFIG.hash.cost).expect("Failed to hash");
        self.conn
            .execute(
                "INSERT INTO users (id, username, email, password) VALUES (?1, ?2, ?3, ?4)",
                params![id, username, email, hashed_pass],
            )
            .unwrap();
        Ok(User {
            id,
            username,
            email,
            role: "user".to_string(),
            verified: false,
        })
    }

    pub fn get_user_by_id(&self, id: String) -> Option<User> {
        self.query_single("SELECT * FROM users WHERE id = ?1", params![id])
    }

    pub fn get_user_by_username(&self, username: &str) -> Option<User> {
        self.query_single("SELECT * FROM users WHERE username = ?1", params![username])
    }

    pub fn get_user_by_email(&self, email: &str) -> Option<User> {
        self.query_single("SELECT * FROM users WHERE email = ?1", params![email])
    }

    pub fn get_user_by_login(&self, login: &str) -> Option<User> {
        self.query_single(
            "SELECT * FROM users WHERE email = ?1 OR username = ?1",
            params![login],
        )
    }

    pub fn verify_login(&self, login: &str, password: &str) -> Option<User> {
        let user = self.get_user_by_login(login)?;
        let db_password = self.get_password_by_id(&user.id)?;
        let valid = verify(password, &db_password).unwrap_or(false);
        if valid { Some(user) } else { None }
    }

    fn query_single(&self, sql: &str, params: impl rusqlite::Params) -> Option<User> {
        let mut stmt = self.conn.prepare(sql).unwrap();
        let mut rows = stmt.query(params).unwrap();
        if let Some(row) = rows.next().unwrap() {
            Some(User {
                id: row.get(0).unwrap(),
                username: row.get(1).unwrap(),
                email: row.get(2).unwrap(),
                role: row.get(3).unwrap(),
                verified: row.get::<_, i64>(5).unwrap() != 0,
            })
        } else {
            None
        }
    }
    pub fn get_password_by_id(&self, id: &String) -> Option<String> {
        self.conn
            .query_row("SELECT password FROM users WHERE id = ?1", [id], |row| {
                row.get(0)
            })
            .ok()
    }
}
