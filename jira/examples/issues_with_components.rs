use jira::models::issue::IssueSummary;
use jira::{Credentials, Jira};
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let host = env::var("JIRA_HOST")?;
    let user = env::var("JIRA_USER")?;
    let token = env::var("JIRA_TOKEN")?;

    let jira =
        Jira::new(&host, Credentials::Basic(user, token)).expect("Error initializing Jira client");

    println!("Searching for all issues with worklogs");
    let issue_summaries: Vec<IssueSummary> = jira
        .fetch_with_jql(
            "component is not empty and worklogAuthor is not empty",
            vec!["key", "summary", "components"],
        )
        .await?;
    assert!(!issue_summaries.is_empty(), "No issues found");
    eprintln!("Found {} issues", issue_summaries.len());

    for issue in &issue_summaries {
        println!("{:8} {}", issue.key, issue.fields.summary);
        for component in &issue.fields.components {
            println!("  - {} {}", component.id, component.name);
        }
    }
    Ok(())
}
