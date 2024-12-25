use jira::models::issue::IssueSummary;
use jira::{Credentials, Jira};
use std::env;
use worklog::ApplicationRuntime;

#[tokio::test]
async fn test_component_insert() -> Result<(), Box<dyn std::error::Error>> {

    let host = env::var("JIRA_HOST")?;
    let user = env::var("JIRA_USER")?;
    let token = env::var("JIRA_TOKEN")?;

    let jira =
        Jira::new(&host, Credentials::Basic(user, token)).expect("Error initializing Jira client");

    let issue_summaries = jira
        .fetch_with_jql::<IssueSummary>(
            "component is not empty and worklogAuthor is not empty",
            vec!["key", "summary", "components"],
        )
        .await?;
    assert!(!issue_summaries.is_empty(), "No issues found");
    eprintln!("Found {} issues", issue_summaries.len());
    env_logger::init();

    let runtime = ApplicationRuntime::new()?;

    for issue in &issue_summaries {
        println!("{:8} {}", issue.key, issue.fields.summary);
        for component in &issue.fields.components {
            println!("  - {} {} (inserting)", component.id, component.name);

            runtime
                .worklog_service()
                .add_component(&issue.key, &issue.fields.components)?;
        }
    }
    Ok(())
}
