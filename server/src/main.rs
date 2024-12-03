use axum::{
    response::{Html, Json},
    routing::get,
    Router,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use tower_http::cors::{Any, CorsLayer};

#[derive(Serialize)]
struct Message {
    message: String,
}

async fn root() -> Html<&'static str> {
    Html("<h1>Welcome to the Timesheet Rust Axum Backend</h1>")
}

async fn api() -> Json<Message> {
    use chrono::Local;

    Json(Message {
        message: format!(
            "Hello from Rust at {}",
            Local::now().format("%Y-%m-%d %H:%M:%S")
        ),
    })
}
use serde_json::json;

#[derive(Serialize, Deserialize, Debug)]
struct Timesheet {
    projects: Vec<String>,
    hours: Vec<Vec<u32>>, // 2D array for hours worked per day and per project
}

// The timesheet handler function
async fn get_timesheet() -> Json<Timesheet> {
    // Create some mock data to represent the current timesheet
    let timesheet = Timesheet {
        projects: vec![
            String::from("TIME-117"),
            String::from("TIME-147"),
            String::from("TIME-148"),
        ],
        hours: vec![
            vec![4, 5, 0, 8, 7, 3, 2], // TIME-117 Monday through Sunday
            vec![5, 1, 4, 0, 3, 1, 4], // TIME-147
            vec![0, 4, 6, 0, 2, 5, 3], // TIME-148
        ],
    };

    // Return the timesheet data as a JSON response
    Json(timesheet)
}

use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::post;

// Handler to handle POST requests to /worklog/timesheet
async fn post_timesheet(Json(payload): Json<Timesheet>) -> impl IntoResponse {
    // Here you can process the timesheet data, such as saving it to a database
    // For now, we will just print the received data
    println!("Received timesheet data: {payload:?}");

    // Return a success message
    (
        StatusCode::OK,
        Json(json!({
            "message": "Timesheet successfully updated",
        })),
    )
}

#[tokio::main]
async fn main() {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/", get(root))
        .route("/api", get(api))
        .route("/worklog/timesheet", get(get_timesheet))
        .route("/worklog/timesheet", post(post_timesheet))
        .layer(cors);

    let addr = SocketAddr::from(([127, 0, 0, 1], 4000));
    println!("Server running on {addr}");

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
