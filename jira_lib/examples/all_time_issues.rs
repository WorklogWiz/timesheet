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
        let issues = jira_client
            .get_issues_for_single_project("TIME".to_string())
            .await;
        for issue in issues {
            println!("{} {}", issue.key, issue.fields.summary);
        }
    }
}
