mod test_helpers;

use crate::test_helpers::jira_client::create_jira_client;
use crate::test_helpers::test_data;
use env_logger::init;
use jira::models::project::JiraProjectKey;

#[tokio::test] // Requires a valid user token in configuration
async fn search_issues_test() -> Result<(), Box<dyn std::error::Error>> {
    init();

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
