use axum::{
    extract::{Json, Query},
    http::{HeaderMap, StatusCode, Method},
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use tower_http::cors::{Any, CorsLayer};

mod config;
use config::CONFIG;
mod database;
use database::DB;
mod tokens;
use crate::tokens::{create_token, verify_token};

#[derive(Deserialize)]
struct UserRegister {
    username: String,
    email: String,
    password: String,
}

#[derive(Deserialize)]
struct UserLogin {
    login: String,
    password: String,
}

#[derive(Serialize, Deserialize)]
struct ErrorJson {
    message: String,
}

#[tokio::main]
async fn main() {
    let config = &CONFIG;
    let cors = CorsLayer::permissive();

    let app = Router::new()
        .route("/register", post(register))
        .route("/login", post(login))
        .route("/verify", post(verify))
        .route("/users", get(get_users))
        .route("/users/search", get(search_users))
        .layer(cors);

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

async fn register(Json(payload): Json<UserRegister>) -> impl IntoResponse {
    let db = DB.lock().unwrap();
    let email_exists = db.get_user_by_email(&payload.email).is_some();
    let username_exists = db.get_user_by_username(&payload.username).is_some();

    if email_exists || username_exists {
        return (
            StatusCode::CONFLICT,
            Json(ErrorJson {
                message: "User already exists".to_string(),
            }),
        )
            .into_response();
    }
    let user = db
        .create_user(payload.username, payload.email, payload.password)
        .unwrap();
    let mut headers = HeaderMap::new();
    let token = create_token(user.id);
    headers.insert(
        "Authorization",
        format!("Bearer {}", token).parse().unwrap(),
    );
    (headers, StatusCode::OK).into_response()
}

async fn login(Json(payload): Json<UserLogin>) -> impl IntoResponse {
    let db = DB.lock().unwrap();
    let user_exists = db.get_user_by_login(&payload.login).is_some();
    if !user_exists {
        return (
            StatusCode::CONFLICT,
            Json(ErrorJson {
                message: "User not exists".to_string(),
            }),
        )
            .into_response();
    }
    let result = db.verify_login(&payload.login, &payload.password);

    if result.is_some() {
        let user = result.unwrap();
        let token = create_token(user.id);
        let mut headers = HeaderMap::new();
        headers.insert(
            "Authorization",
            format!("Bearer {}", token).parse().unwrap(),
        );
        (headers, StatusCode::OK).into_response()
    } else {
        StatusCode::UNAUTHORIZED.into_response()
    }
}
async fn verify(headers: HeaderMap) -> impl IntoResponse {
    let db = DB.lock().unwrap();
    if let Some(auth_header) = headers.get("Authorization") {
        if let Ok(header_value) = auth_header.to_str() {
            let token = header_value
                .strip_prefix("Bearer ")
                .unwrap_or(header_value)
                .trim();
            let userid_opt = verify_token(token);
            match userid_opt {
                Some(userid) => {
                    let resp = db.get_user_by_id(userid);
                    match resp {
                        Some(user) => (StatusCode::OK, Json(user)).into_response(),
                        None => (
                            StatusCode::NOT_FOUND,
                            Json(ErrorJson {
                                message: "User not found".to_string(),
                            }),
                        )
                            .into_response(),
                    }
                }
                None => (
                    StatusCode::UNAUTHORIZED,
                    Json(ErrorJson {
                        message: "Invalid/Expired Token".to_string(),
                    }),
                )
                    .into_response(),
            }
        } else {
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    } else {
        StatusCode::BAD_REQUEST.into_response()
    }
}

async fn get_users() -> impl IntoResponse {
    let db_lock = DB.lock().unwrap();
    let users = db_lock.get_users();
    (StatusCode::OK, Json(users)).into_response()
}

#[derive(Deserialize)]
struct SearchQuery {
    q: String,
    limit: Option<i32>,
}

async fn search_users(Query(query): Query<SearchQuery>) -> impl IntoResponse {
    let db_lock = DB.lock().unwrap();
    let limit = query.limit.unwrap_or(20);
    let users = db_lock.search_users(&query.q, limit);
    (StatusCode::OK, Json(users)).into_response()
}
