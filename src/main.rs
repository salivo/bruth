use axum::{Json, Router, http::StatusCode, response::IntoResponse, routing::post};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

mod config;
use config::CONFIG;
mod database;
use database::DB;
mod tokens;
use tokens::TOKENS;

#[derive(Deserialize)]
struct UserRequest {
    username: String,
    email: String,
    password: String,
}

#[derive(Serialize)]
struct UserAnswer {
    username: String,
    email: String,
    verified: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct TokenJson {
    token: String,
}

#[derive(Serialize, Deserialize)]
struct UserIdJson {
    id: String,
}

#[derive(Serialize, Deserialize)]
struct ErrorJson {
    message: String,
}
#[tokio::main]
async fn main() {
    let config = &CONFIG;
    let app = Router::new()
        .route("/register", post(create_user))
        .route("/user", post(get_user))
        .route("/verify", post(verify_user));
    let host_addr: [u8; 4] = config
        .main
        .host
        .split('.')
        .map(|x| x.parse::<u8>().unwrap())
        .collect::<Vec<u8>>()
        .try_into()
        .unwrap();
    let addr = SocketAddr::from((host_addr, config.main.port));
    println!("Server listening on http://{}/", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn get_user(Json(payload): Json<UserIdJson>) -> impl IntoResponse {
    let db = DB.lock().unwrap();
    let result = db.get_user_by_id(payload.id).unwrap();
    match result {
        Some(user) => (
            StatusCode::OK,
            Json(UserAnswer {
                username: user.username,
                email: user.email,
                verified: user.verified,
            }),
        )
            .into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(ErrorJson {
                message: "User not found".to_string(),
            }),
        )
            .into_response(),
    }
}

async fn verify_user(Json(payload): Json<TokenJson>) -> impl IntoResponse {
    println!("Token verification result: {:?}", payload);
    let tm = TOKENS.lock().unwrap();
    let result = tm.validate_token(&payload.token);
    match result {
        Some(_id) => (StatusCode::OK, Json(UserIdJson { id: _id.clone() })).into_response(),
        None => (
            StatusCode::UNAUTHORIZED,
            Json(ErrorJson {
                message: "Unauthorized".to_string(),
            }),
        )
            .into_response(),
    }
}

async fn create_user(Json(payload): Json<UserRequest>) -> impl IntoResponse {
    let db = DB.lock().unwrap();
    let email_exists = db.get_user_by_email(&payload.email).unwrap().is_some();
    let username_exists = db
        .get_user_by_username(&payload.username)
        .unwrap()
        .is_some();

    if email_exists || username_exists {
        (
            StatusCode::CONFLICT,
            Json(ErrorJson {
                message: "User already exists".to_string(),
            }),
        )
            .into_response()
    } else {
        let id = db
            .create_user(payload.username, payload.email, payload.password)
            .unwrap();
        let mut tm = TOKENS.lock().unwrap();
        (
            StatusCode::OK,
            Json(TokenJson {
                token: { tm.create_token(id) },
            }),
        )
            .into_response()
    }
}
