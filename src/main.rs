use axum::{
    Router,
    extract::{Json, Query},
    http::{
        HeaderMap, HeaderValue, Method, Request, StatusCode,
        header::{AUTHORIZATION, CONTENT_TYPE},
    },
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use tower_http::cors::CorsLayer;

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

fn validate_register(payload: &UserRegister) -> Result<(), String> {
    let v = &CONFIG.validation;

    let username = payload.username.trim();
    if username.is_empty() || username.len() > v.username_max_len {
        return Err(format!(
            "Username must be between 1 and {} characters",
            v.username_max_len
        ));
    }

    if payload.email.len() > v.email_max_len
        || !payload.email.contains('@')
        || payload
            .email
            .split('@')
            .nth(1)
            .map_or(true, |domain| !domain.contains('.'))
    {
        return Err("Invalid email address".to_string());
    }

    if payload.password.len() < v.password_min_len {
        return Err(format!(
            "Password must be at least {} characters",
            v.password_min_len
        ));
    }

    if payload.password.len() > v.password_max_len {
        return Err(format!(
            "Password must not exceed {} characters",
            v.password_max_len
        ));
    }

    Ok(())
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter()
        .zip(b.iter())
        .fold(0u8, |acc, (x, y)| acc | (x ^ y))
        == 0
}

async fn require_service_key<B>(headers: HeaderMap, req: Request<B>, next: Next<B>) -> Response
where
    B: Send + 'static,
{
    let expected =
        std::env::var("SERVICE_API_KEY").expect("SERVICE_API_KEY environment variable must be set");

    let provided = headers
        .get("X-Api-Key")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if constant_time_eq(provided.as_bytes(), expected.as_bytes()) {
        next.run(req).await
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

#[tokio::main]
async fn main() {
    let config = &CONFIG;

    let _jwt_secret =
        std::env::var("JWT_SECRET").expect("JWT_SECRET environment variable must be set");
    let _service_key =
        std::env::var("SERVICE_API_KEY").expect("SERVICE_API_KEY environment variable must be set");

    let allowed_origin: HeaderValue = std::env::var("CORS_ALLOWED_ORIGIN")
        .expect("CORS_ALLOWED_ORIGIN environment variable must be set")
        .parse()
        .expect("CORS_ALLOWED_ORIGIN is not a valid header value");

    let cors = CorsLayer::new()
        .allow_origin(allowed_origin)
        .allow_methods([Method::GET, Method::POST])
        .allow_headers([AUTHORIZATION, CONTENT_TYPE]);

    let service_routes = Router::new()
        .route("/users", get(get_users))
        .route("/users/search", get(search_users))
        .route_layer(middleware::from_fn(require_service_key));

    let app = Router::new()
        .route("/register", post(register))
        .route("/login", post(login))
        .route("/verify", post(verify))
        .merge(service_routes)
        .layer(cors);

    let host_addr: [u8; 4] = config
        .main
        .host
        .split('.')
        .map(|x| x.parse::<u8>().unwrap())
        .collect::<Vec<u8>>()
        .try_into()
        .unwrap();
    let addr = SocketAddr::from((host_addr, 8080u16));
    println!("Server listening on http://{}/", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn register(Json(payload): Json<UserRegister>) -> impl IntoResponse {
    if let Err(msg) = validate_register(&payload) {
        return (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(ErrorJson { message: msg }),
        )
            .into_response();
    }

    let db = DB.lock().unwrap_or_else(|e| e.into_inner());
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
    let token = create_token(user.id);
    let mut headers = HeaderMap::new();
    headers.insert(
        "Authorization",
        format!("Bearer {}", token).parse().unwrap(),
    );
    (headers, StatusCode::OK).into_response()
}

async fn login(Json(payload): Json<UserLogin>) -> impl IntoResponse {
    let db = DB.lock().unwrap_or_else(|e| e.into_inner());
    match db.verify_login(&payload.login, &payload.password) {
        Some(user) => {
            let token = create_token(user.id);
            let mut headers = HeaderMap::new();
            headers.insert(
                "Authorization",
                format!("Bearer {}", token).parse().unwrap(),
            );
            (headers, StatusCode::OK).into_response()
        }
        None => (
            StatusCode::UNAUTHORIZED,
            Json(ErrorJson {
                message: "Invalid credentials".to_string(),
            }),
        )
            .into_response(),
    }
}

async fn verify(headers: HeaderMap) -> impl IntoResponse {
    let db = DB.lock().unwrap_or_else(|e| e.into_inner());
    let Some(auth_header) = headers.get("Authorization") else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    let Ok(header_value) = auth_header.to_str() else {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };

    let token = header_value
        .strip_prefix("Bearer ")
        .unwrap_or(header_value)
        .trim();

    match verify_token(token) {
        Some(userid) => match db.get_user_by_id(userid) {
            Some(user) => (StatusCode::OK, Json(user)).into_response(),
            None => (
                StatusCode::NOT_FOUND,
                Json(ErrorJson {
                    message: "User not found".to_string(),
                }),
            )
                .into_response(),
        },
        None => (
            StatusCode::UNAUTHORIZED,
            Json(ErrorJson {
                message: "Invalid/Expired Token".to_string(),
            }),
        )
            .into_response(),
    }
}

async fn get_users() -> impl IntoResponse {
    let db_lock = DB.lock().unwrap_or_else(|e| e.into_inner());
    (StatusCode::OK, Json(db_lock.get_users())).into_response()
}

#[derive(Deserialize)]
struct SearchQuery {
    q: String,
    limit: Option<i32>,
}

async fn search_users(Query(query): Query<SearchQuery>) -> impl IntoResponse {
    let max = CONFIG.validation.search_limit_max;
    let limit = query.limit.unwrap_or(20).clamp(1, max);
    let db_lock = DB.lock().unwrap_or_else(|e| e.into_inner());
    (StatusCode::OK, Json(db_lock.search_users(&query.q, limit))).into_response()
}
