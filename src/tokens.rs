use chrono::{Duration, Utc};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};

use crate::config::CONFIG;

#[derive(Debug, Serialize, Deserialize)]
pub struct Payload {
    pub sub: String,
    pub exp: usize,
}

pub fn create_token(id: String) -> String {
    let expiration = Utc::now()
        .checked_add_signed(Duration::seconds(CONFIG.token.duration))
        .expect("valid timestamp")
        .timestamp();

    let payload = Payload {
        sub: id,
        exp: expiration as usize,
    };

    let secret_bytes = CONFIG.token.secret.as_bytes();

    encode(
        &Header::default(),
        &payload,
        &EncodingKey::from_secret(secret_bytes),
    )
    .expect("Token creation failed")
}

pub fn verify_token(token: &str) -> Option<String> {
    let validation = Validation::new(Algorithm::HS256);

    let secret_bytes = CONFIG.token.secret.as_bytes();

    let token_data = decode::<Payload>(token, &DecodingKey::from_secret(secret_bytes), &validation);

    match token_data {
        Ok(c) => Some(c.claims.sub), // return userid
        Err(_) => None,
    }
}
