use dict::{Dict, DictIface};
use once_cell::sync::Lazy;
use std::sync::Mutex;
use uuid::Uuid;

pub static TOKENS: Lazy<Mutex<TokenManager>> = Lazy::new(|| {
    let tm = TokenManager::new();
    Mutex::new(tm)
});

pub struct TokenManager {
    tokens: Dict<String>,
}

impl TokenManager {
    fn new() -> Self {
        TokenManager {
            tokens: Dict::new(),
        }
    }
    pub fn create_token(&mut self, id: String) -> String {
        let token = Uuid::new_v4().to_string();
        self.tokens.add(token.clone(), id);
        token
    }
    pub fn validate_token(&self, token: &str) -> Option<&String> {
        self.tokens.get(token)
    }
    pub fn delete_token(&mut self, token: &str) -> Result<(), String> {
        if self.tokens.contains_key(token) {
            self.tokens.remove_key(token);
            Ok(())
        } else {
            Err("Already logged out".to_string())
        }
    }
}
