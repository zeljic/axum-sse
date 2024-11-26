use axum::extract::{Path, State};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

mod config;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct User {
    name: String,
    age: u8,
}

#[derive(Clone)]
struct AppState {
    user: Arc<Mutex<User>>,
}

async fn index() -> &'static str {
    "zdravo svete"
}

async fn get_user(State(app_state): State<AppState>) -> Result<impl IntoResponse, &'static str> {
    match app_state.user.try_lock() {
        Ok(user) => Ok(axum::Json(user.clone())),
        Err(e) => {
            eprintln!("Error: {:?}", e);

            Err("Error")
        }
    }
}

async fn fallback() -> impl IntoResponse {
    (axum::http::StatusCode::NOT_FOUND, "Not Found")
}

async fn change_age(State(app_state): State<AppState>, Path(v): Path<u8>) -> impl IntoResponse {
    let mut user = app_state.user.lock().unwrap();
    user.age = v;

    axum::http::StatusCode::OK
}

async fn change_name(
    State(app_state): State<AppState>,
    Path(v): Path<String>,
) -> impl IntoResponse {
    let mut user = app_state.user.lock().unwrap();
    user.name = v.clone();

    axum::http::StatusCode::OK
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = config::Config::new();

    let user = User {
        name: "zeljic".to_string(),
        age: 38,
    };

    let app_state = AppState {
        user: Arc::new(Mutex::new(user)),
    };

    let router = axum::Router::new()
        .route("/", get(index))
        .route("/user", get(get_user))
        .route("/age/:age", get(change_age))
        .route("/name/:name", get(change_name))
        .with_state(app_state)
        .fallback(fallback);

    let listener =
        tokio::net::TcpListener::bind(format!("{}:{}", config.server_host, config.server_port))
            .await?;

    axum::serve(listener, router.into_make_service()).await?;

    Ok(())
}
