use std::env;

use jira::{Credentials, Jira};

#[tokio::main]
async fn main() {
    if let (Ok(host), Ok(user), Ok(token)) = (
        env::var("JIRA_HOST"),
        env::var("JIRA_USER"),
        env::var("JIRA_TOKEN"),
    ) {
        let jira_client = Jira::new(&host, Credentials::Basic(user, token))
            .expect("Error initializing jira client");
        let dt = chrono::offset::Local::now();

        println!("Executing ...");
        let _r = jira_client
            .insert_worklog("TIME-94", dt, 27000, "Rubbish comment")
            .await;
    } else {
        panic!("Missing env var JIRA_HOST, JIRA_USER or JIRA_TOKEN")
    }
}
