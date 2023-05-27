use chrono::{DateTime, Utc};
use jira_lib::{http_client};

#[tokio::main]
async fn main() {
    // Creates HTTP client with all the required credentials
    let http_client = http_client();

    let dt = chrono::offset::Local::now();

    println!("Executing ...");
    let r = jira_lib::insert_worklog(&http_client, "TIME-94", dt, 27000, "Rubbish comment").await;
}

