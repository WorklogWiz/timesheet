mod test_helpers;

use crate::test_helpers::jira_client::create_jira_client;
use jira::models::project::JiraProjectKey;
use log::debug;
use std::string::ToString;

#[tokio::test] // Requires a valid user token in configuration
async fn test_create_issue() -> Result<(), Box<dyn std::error::Error>> {
    let jira_client = create_jira_client().await;
    let new_issue = jira_client
        .create_issue(
            JiraProjectKey {
                key: "NOR".to_string(),
            },
            "Test issue",
            Some("Test description".to_string()),
        )
        .await
        .unwrap();
    assert!(!new_issue.key.is_empty());
    debug!("Created issue {}", new_issue.key);

    Ok(())
}
