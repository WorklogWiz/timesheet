mod test_helpers;

use crate::test_helpers::jira_client::create_jira_client;
use env_logger::init;
use jira::models::core::JiraKey;
use jira::models::project::JiraProjectKey;
use log::debug;
use std::string::ToString;
use test_helpers::test_data;

#[tokio::test] // Requires a valid user token in configuration
async fn test_create_issue() -> Result<(), Box<dyn std::error::Error>> {
    let jira_client = create_jira_client().await;
    let new_issue = jira_client
        .create_issue(
            &JiraProjectKey { key: "NOR" },
            "Test issue",
            Some("Test description".to_string()),
        )
        .await
        .unwrap();
    assert!(!new_issue.key.is_empty());
    debug!("Created issue {}", new_issue.key);

    Ok(())
}

#[tokio::test]
async fn create_multiple_issues_slow_version() -> Result<(), Box<dyn std::error::Error>> {
    let jira_client = create_jira_client().await;
    let mut issues = Vec::new();
    for _ in 0..10 {
        let new_issue = jira_client
            .create_issue(
                &JiraProjectKey { key: "NOR" },
                "Test issue",
                Some("Test description".to_string()),
            )
            .await?;
        assert!(!new_issue.key.is_empty());
        debug!("Created issue {}", new_issue.key);
        let jira_key = JiraKey::from(new_issue.key);
        issues.push(jira_key);
    }
    assert!(!issues.is_empty());
    for jira_key in issues {
        debug!("Deleting issue {}", jira_key);
        jira_client.delete_issue(&jira_key).await?;
    }
    Ok(())
}

#[tokio::test]
async fn create_multiple_issues_fast_version() -> Result<(), Box<dyn std::error::Error>> {
    init();
    let issue_keys = test_data::create_batch_of_issues(10, JiraProjectKey { key: "NOR" }).await?;
    debug!("Created {} issues", issue_keys.len());
    assert!(!issue_keys.is_empty());
    debug!("Deleting the issues");
    test_data::delete_batch_of_issues_by_key(&issue_keys).await;
    Ok(())
}
