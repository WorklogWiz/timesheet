mod test_helpers;

use crate::test_helpers::jira_client::create_jira_client;
use crate::test_helpers::test_data::TEST_PROJECT_KEY;
use jira::models::project::JiraProjectKey;
use log::debug;
use std::string::ToString;
use test_helpers::test_data;

#[tokio::test] // Requires a valid user token in configuration
async fn test_create_issue() -> Result<(), Box<dyn std::error::Error>> {
    let jira_client = create_jira_client().await;
    let new_issue = jira_client
        .create_issue(
            &JiraProjectKey {
                key: TEST_PROJECT_KEY,
            },
            "Test issue",
            Some("Test description".to_string()),
            vec![],
        )
        .await
        .unwrap();
    assert!(!new_issue.key.is_empty());
    debug!("Created issue {}", new_issue.key);

    jira_client.delete_issue(&new_issue.key).await?;
    debug!("Deleted issue {}", new_issue.key);
    Ok(())
}

#[tokio::test]

async fn create_multiple_issues_fast_version() -> Result<(), Box<dyn std::error::Error>> {
    let issue_keys = test_data::create_batch_of_issues(
        10,
        JiraProjectKey {
            key: TEST_PROJECT_KEY,
        },
    )
    .await?;
    debug!("Created {} issues", issue_keys.len());
    assert!(!issue_keys.is_empty());
    debug!("Deleting the issues");
    test_data::delete_batch_of_issues_by_key(&issue_keys).await;
    Ok(())
}
