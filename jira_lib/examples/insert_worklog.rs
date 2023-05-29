use chrono::{DateTime, Utc};
use jira_lib::{http_client};

#[tokio::main]
async fn main() {
    let jira_client = jira_lib::create_jira_client();


    let dt = chrono::offset::Local::now();

    println!("Executing ...");
    let r = jira_lib::insert_worklog(&jira_client.http_client, "TIME-94", dt, 27000, "Rubbish comment").await;
}

