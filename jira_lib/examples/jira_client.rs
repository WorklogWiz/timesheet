use std::env;

#[tokio::main]
async fn main() {
    env_logger::init();
    if let (Ok(host), Ok(user), Ok(token)) = (
        env::var("JIRA_HOST"),
        env::var("JIRA_USER"),
        env::var("JIRA_TOKEN"),
    ) {
        let jira_client = jira_lib::Jira::new(
            &format!("{host}/rest/api/latest"),
            &user,
            &token).expect("Error initializing jira client");
        let _results = jira_client.get_time_tracking_options().await;
    }
}
