use chrono::{DateTime, Utc};

#[tokio::main]
async fn main() {
    let jira_client = jira_lib::create_jira_client();


    let dt = chrono::offset::Local::now();

    println!("Executing ...");
    let r = jira_client.insert_worklog( "TIME-94", dt, 27000, "Rubbish comment").await;
}

