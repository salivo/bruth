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
    id: String,
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
        .route("/logout", post(logout));
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

async fn get_user(Json(payload): Json<TokenJson>) -> impl IntoResponse {
    let tm = TOKENS.lock().unwrap();
    let db = DB.lock().unwrap();
    let result = tm.validate_token(&payload.token);
    if let Some(id) = result {
        let result = db.get_user_by_id(id.clone()).unwrap();
        match result {
            Some(user) => (
                StatusCode::OK,
                Json(UserAnswer {
                    id: user.id,
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
    } else {
        (
            StatusCode::UNAUTHORIZED,
            Json(ErrorJson {
                message: "Unauthorized".to_string(),
            }),
        )
            .into_response()
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

async fn logout(Json(payload): Json<TokenJson>) -> impl IntoResponse {
    let mut tm = TOKENS.lock().unwrap();
    let result = tm.delete_token(&payload.token);
    match result {
        Ok(_) => (StatusCode::OK).into_response(),
        Err(err) => (StatusCode::CONFLICT, Json(ErrorJson { message: err })).into_response(),
    }
}
