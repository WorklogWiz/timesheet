mod test_helpers;

use crate::test_helpers::jira_client::create_jira_client;
use crate::test_helpers::test_data;

use jira::models::project::JiraProjectKey;

/// This asynchronous test verifies the Jira client functionality for searching issues.
///
/// It performs the following steps:
/// 1. Creates a batch of issues in Jira for a specific project key.
/// 2. Adds random work logs to these issues, simulating real-world usage.
/// 3. Searches for issues using the Jira client, with the same project key.
/// 4. Confirms that the search result contains at least the number of issues created.
/// 5. Cleans up by deleting the created issues.
///
/// # Requirements:
/// - A valid user token must be configured for the Jira client (as indicated by `#[tokio::test]`).
///
/// # Errors:
/// This test will return an error if:
/// - Issue creation, logging, searching, or deletion fails.
/// - The search result contains fewer issues than were created.
///
/// # Panics:
/// The test will panic if the assertion comparing the number of created and found issues fails.
///
/// # Returns:
/// - Returns `Ok(())` on success.
#[tokio::test] // Requires a valid user token in configuration
async fn search_issues_test() -> Result<(), Box<dyn std::error::Error>> {
    let issues = test_data::create_batch_of_issues(3, JiraProjectKey { key: "NOR" }).await?;

    let _work_logs = test_data::add_random_work_logs_to_issues(&issues, 1..3).await;

    let jira_client = create_jira_client().await;
    let search_result = jira_client.search_issues(&vec!["NOR"], &vec![]).await?;
    assert!(!issues.is_empty());

    // We expect at least same number of issues when we search
    assert!(
        issues.len() <= search_result.len(),
        "Search for issues returned less than expected. Created {}, found {}",
        issues.len(),
        search_result.len()
    );

    test_data::delete_batch_of_issues_by_key(&issues).await;
    Ok(())
}
