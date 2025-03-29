use std::env;

use jira::{models::setting::GlobalSettings, Credentials, Jira};

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

        let results = client.get::<GlobalSettings>("/configuration").await;
        println!("{results:?}");
    } else {
        panic!("Missing env var JIRA_HOST, JIRA_USER or JIRA_TOKEN")
    }
}
