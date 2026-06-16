use chrono::{Duration, Utc};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};

use crate::config::CONFIG;

#[derive(Debug, Serialize, Deserialize)]
pub struct Payload {
    pub sub: String,
    pub exp: usize,
}

fn jwt_secret() -> String {
    std::env::var("JWT_SECRET").expect("JWT_SECRET environment variable must be set")
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

    let secret = jwt_secret();
    encode(
        &Header::default(),
        &payload,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .expect("Token creation failed")
}

pub fn verify_token(token: &str) -> Option<String> {
    let validation = Validation::new(Algorithm::HS256);
    let secret = jwt_secret();
    let token_data = decode::<Payload>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    );

    match token_data {
        Ok(c) => Some(c.claims.sub),
        Err(_) => None,
    }
}
