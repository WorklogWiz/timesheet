use std::env;

use jira::{Credentials, Jira};

#[tokio::main]
async fn main() {
    std::env::set_var("RUST_LOG", "debug");
    env_logger::init();

    if let (Ok(host), Ok(user), Ok(token)) = (
        env::var("JIRA_HOST"),
        env::var("JIRA_USER"),
        env::var("JIRA_TOKEN"),
    ) {
        let client = Jira::new(&host, Credentials::Basic(user, token))
            .expect("Error initializing jira client");
        let issues = client
            .get_issues_for_project("TIME".to_string())
            .await
            .expect("Failed to get projects");
        for issue in issues {
            println!("{} {}", issue.key, issue.fields.summary);
        }
    } else {
        panic!("Missing env var JIRA_HOST, JIRA_USER or JIRA_TOKEN")
    }
}
