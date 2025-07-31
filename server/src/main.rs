use axum::routing::{get, post};
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Json, Response},
    Router,
};
use chrono::{Duration, Local};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Mutex;
use worklog::{error::WorklogError, types::LocalWorklog, ApplicationRuntime};

use serde_json::json;

use thiserror::Error;
use tower_http::cors::{Any, CorsLayer};

#[derive(Error, Debug)]
pub enum ServerError {
    #[error("Internal Server Error")]
    InternalServerError,
    #[error("Bad Request")]
    BadRequest,
    #[error("Worklog error")]
    WorklogError(#[from] WorklogError),
}

impl IntoResponse for ServerError {
    fn into_response(self) -> Response {
        let status_code = match self {
            ServerError::InternalServerError => StatusCode::INTERNAL_SERVER_ERROR,
            ServerError::BadRequest => StatusCode::BAD_REQUEST,
            ServerError::WorklogError(_worklog_error) => StatusCode::INTERNAL_SERVER_ERROR,
        };

        let message = "Something went wrong".to_string();
        (status_code, message).into_response()
    }
}

async fn get_worklogs(
    State(state): State<AppState>,
) -> Result<Json<Vec<LocalWorklog>>, ServerError> {
    // TODO: Consider removing this as the ApplicationRuntime should be thread safe now.
    let runtime = state.runtime.lock().await;

    let keys = runtime.issue_service().find_unique_keys()?;
    let worklogs = runtime.worklog_service().find_worklogs_after(
        Local::now()
            .checked_sub_signed(Duration::days(365))
            .unwrap(),
        &keys,
        &[],
    )?; // Use public method to avoid referencing private type

    // Return the timesheet data as a JSON response
    Ok(Json(worklogs))
}

// Handler to handle POST requests to /worklog/timesheet
async fn post_worklog(Json(payload): Json<LocalWorklog>) -> impl IntoResponse {
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

#[derive(Clone)]
struct AppState {
    // TODO: consider removing this, as ApplicationRuntime is now thread safe
    runtime: Arc<Mutex<ApplicationRuntime>>,
}

#[tokio::main]
async fn main() -> Result<(), ServerError> {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);
    let state = AppState {
        runtime: Arc::new(Mutex::new(ApplicationRuntime::new()?)),
    };

    let app = Router::new()
        .route("/api/worklogs", get(get_worklogs))
        .route("/api/worklogs", post(post_worklog))
        //.route("/api/tracking", get(get_tracking_candidates))
        .with_state(state)
        .layer(cors);

    let addr = SocketAddr::from(([127, 0, 0, 1], 4000));
    println!("Server running on http://{addr}");

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();

    Ok(())
}
