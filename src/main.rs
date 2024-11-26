use axum::extract::{Path, State};
use axum::response::sse::{Event, KeepAlive};
use axum::response::{IntoResponse, Sse};
use axum::routing::get;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::convert::Infallible;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::{Stream, StreamExt};
use tower_http::services::ServeDir;

mod config;

#[derive(Debug, Clone, Serialize, Deserialize)]
enum Notification {
    NameChanged(String),
    AgeChanged(u8),
    Error(String),
}

impl From<Notification> for serde_json::Value {
    fn from(value: Notification) -> serde_json::Value {
        match value {
            Notification::NameChanged(name) => {
                json!({ "kind": "name_changed", "value": {"name": name} })
            }
            Notification::AgeChanged(age) => {
                json!({ "kind": "age_changed", "value": {"age": age} })
            }
            Notification::Error(_) => {
                json!({ "kind": "error", "value": {} })
            }
        }
    }
}

impl From<Notification> for Event {
    fn from(value: Notification) -> Event {
        let value: serde_json::Value = value.into();

        Event::default().json_data(value).unwrap_or_else(|_| {
            Event::default()
                .json_data(json!({ "kind": "error" }))
                .unwrap()
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct User {
    name: String,
    age: u8,
}

#[derive(Clone)]
struct AppState {
    user: Arc<Mutex<User>>,
    tx: tokio::sync::broadcast::Sender<Notification>,
}

async fn index() -> &'static str {
    "zdravo svete"
}

async fn fallback() -> impl IntoResponse {
    (axum::http::StatusCode::NOT_FOUND, "Not Found")
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

async fn change_age(State(app_state): State<AppState>, Path(v): Path<u8>) -> impl IntoResponse {
    let mut user = app_state.user.lock().unwrap();
    user.age = v;

    tokio::spawn(async move {
        let _ = app_state.tx.send(Notification::AgeChanged(v));
    });

    axum::http::StatusCode::OK
}

async fn change_name(
    State(app_state): State<AppState>,
    Path(v): Path<String>,
) -> impl IntoResponse {
    let mut user = app_state.user.lock().unwrap();
    user.name = v.clone();

    tokio::spawn(async move {
        let _ = app_state.tx.send(Notification::NameChanged(v));
    });

    axum::http::StatusCode::OK
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = config::Config::new();

    let user = User {
        name: "zeljic".to_string(),
        age: 37,
    };

    let (tx, _) = tokio::sync::broadcast::channel(5);

    let app_state = AppState {
        user: Arc::new(Mutex::new(user)),
        tx,
    };

    let router = axum::Router::new()
        .route("/user", get(get_user))
        .route("/age/:age", get(change_age))
        .route("/name/:name", get(change_name))
        .route("/sse", get(sse_handler))
        .with_state(app_state)
        .nest_service("/", ServeDir::new("assets"))
        .fallback(fallback);

    let listener =
        tokio::net::TcpListener::bind(format!("{}:{}", config.server_host, config.server_port))
            .await?;

    axum::serve(listener, router.into_make_service()).await?;

    Ok(())
}

async fn sse_handler(
    State(app_state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let rx = app_state.tx.subscribe();

    let stream = BroadcastStream::new(rx).map(|e| {
        return match e {
            Ok(notification) => Ok(notification.into()),
            Err(e) => {
                eprintln!("Error: {:?}", e);

                Ok(Notification::Error("Error".to_string()).into())
            }
        };
    });

    Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(5))
            .text("keep-alive"),
    )
}
