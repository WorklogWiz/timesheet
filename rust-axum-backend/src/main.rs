use axum::{
    routing::get,
    Router,
    response::{Html, Json},
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use tokio;
use tower_http::cors::{CorsLayer, Any};

#[derive(Serialize)]
struct Message {
    message: String,
}

async fn root() -> Html<&'static str> {
    Html("<h1>Welcome to the Rust Axum Backend</h1>")
}

async fn api() -> Json<Message> {
    use chrono::Local;

    Json(Message {
        message: format!("Hello from Rust at {}", Local::now().format("%Y-%m-%d %H:%M:%S")),
    })
}
use serde_json::json;

#[derive(Serialize, Deserialize,Debug)]
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
            vec![4, 2, 3], // Monday: Hours for Project A, Project B, Project C
            vec![5, 1, 0], // Tuesday
            vec![0, 4, 6], // Wednesday
            vec![8, 0, 0], // Thursday
            vec![7, 3, 2], // Friday
            vec![3, 1, 5], // Saturday
            vec![2, 4, 3], // Sunday
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
    println!("Received timesheet data: {:?}", payload);

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
    println!("Server running on {}", addr);

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
