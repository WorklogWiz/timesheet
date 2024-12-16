mod test_helpers;

use crate::test_helpers::jira_client::create_jira_client;
use env_logger::init;
use jira::models::core::JiraKey;
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

#[tokio::test]
async fn create_multiple_issues_slow_version() -> Result<(), Box<dyn std::error::Error>> {
    let jira_client = create_jira_client().await;
    let mut issues = Vec::new();
    for _ in 0..10 {
        let new_issue = jira_client
            .create_issue(
                JiraProjectKey {
                    key: "NOR".to_string(),
                },
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

use futures::stream::{FuturesUnordered, StreamExt};

#[tokio::test]
async fn create_multiple_issues_fast_version() -> Result<(), Box<dyn std::error::Error>> {
    init();
    let issue_keys = create_batch_of_issues(10).await?;
    debug!("Created {} issues", issue_keys.len());
    assert!(!issue_keys.is_empty());
    debug!("Deleting the issues");
    delete_batch_of_issues_by_key(&issue_keys).await;
    Ok(())
}

async fn create_batch_of_issues(qty: i32) -> Result<Vec<JiraKey>, Box<dyn std::error::Error>> {
    let jira_client = create_jira_client().await;

    // Create a stream of futures to process in parallel
    let mut issue_futures = FuturesUnordered::new();

    let start_time = std::time::Instant::now();
    for _ in 0..qty {
        let jira_client_clone = jira_client.clone(); // Clone the client to use in each task

        // Task must return a Result
        issue_futures.push(async move {
            let new_issue = jira_client_clone
                .create_issue(
                    JiraProjectKey {
                        key: "NOR".to_string(),
                    },
                    "Test issue",
                    Some("Test description".to_string()),
                )
                .await;
            debug!("Created issue");
            match new_issue {
                Ok(issue) => {
                    assert!(!issue.key.is_empty());
                    debug!("Created issue {}", issue.key);
                    Ok(JiraKey::from(issue.key))
                }
                Err(e) => {
                    eprintln!("Failed to create issue: {}", e);
                    Err(e)
                }
            }
        });
    }

    let mut issues = Vec::new();

    while let Some(result) = issue_futures.next().await {
        match result {
            Ok(ok_result) => issues.push(ok_result),
            Err(e) => eprintln!("Task failed: {e}"), // Handle task panics
        }
    }

    let elapsed_time = start_time.elapsed();
    debug!("Elapsed time: {:?}", elapsed_time);
    Ok(issues)
}

async fn delete_batch_of_issues_by_key(issue_keys: &Vec<JiraKey>) {
    let start_time = std::time::Instant::now();

    let mut delete_futures = FuturesUnordered::new();

    // Add futures for deleting each issue to the FuturesUnordered stream
    for jira_key in issue_keys {
        let jira_client = create_jira_client().await;

        debug!("Preparing to delete issue {}", jira_key);

        delete_futures.push(async move {
            jira_client.delete_issue(jira_key).await.map(|_| {
                debug!("Deleted issue");
            })
        });
    }

    // Consume the futures as they complete
    while let Some(result) = delete_futures.next().await {
        if let Err(e) = result {
            eprintln!("Failed to delete an issue: {}", e);
        }
    }
    let elapsed = start_time.elapsed();
    debug!("Elapsed time: {:?}", elapsed);
}
