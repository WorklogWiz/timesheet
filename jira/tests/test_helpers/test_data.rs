use crate::test_helpers::jira_client::create_jira_client;
use chrono::{DateTime, Duration, Local};
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use jira::models::core::IssueKey;
// Ensure JiraProjectKey is defined to derive Clone
use jira::models::issue::ComponentId;
use jira::models::project::JiraProjectKey;
use jira::models::worklog::Worklog;
use jira::Jira;
use log::debug;
use rand::{thread_rng, Rng};
use std::ops::Range;

/// The constant `TEST_PROJECT_KEY` represents the key of the test project
/// in the Jira environment used for testing purposes.
///
/// This constant is useful for testing functions that require a Jira project key.
///
/// # Example
///
/// ```rust
/// assert_eq!(TEST_PROJECT_KEY, "TWIZ");
/// ```
#[allow(dead_code)]
pub const TEST_PROJECT_KEY: &str = "TWIZ";

// Constant for reuse
#[allow(dead_code)]
const TEST_ISSUE_TYPE: &str = "Task";
#[allow(dead_code)]
const TEST_ISSUE_TITLE: &str = "Test issue";

/// Helper function to create a Jira issue asynchronously
/// Extracted for reusability and code clarity.
#[allow(dead_code)]
async fn create_issue_task(
    jira_client: Jira,
    project_key: JiraProjectKey,
    components: Vec<ComponentId>,
) -> Result<IssueKey, Box<dyn std::error::Error>> {
    let issue_response = jira_client
        .create_issue(
            &project_key,
            TEST_ISSUE_TITLE,
            Some("create_batch_of_issues()".to_string()),
            components,
        )
        .await;

    match issue_response {
        Ok(issue) => {
            assert!(!issue.key.is_empty());
            debug!("Created issue: {}", issue.key);
            Ok(IssueKey::from(issue.key))
        }
        Err(e) => {
            eprintln!("Failed to create issue: {}", e);
            Err(e.into())
        }
    }
}

/// Creates a batch of issues for a specific Jira project.
#[allow(dead_code)]
pub async fn create_batch_of_issues(
    qty: i32,
    jira_project_key: JiraProjectKey,
) -> Result<Vec<IssueKey>, Box<dyn std::error::Error>> {
    let jira_client = create_jira_client().await;

    // Fetch the first component if available
    let first_component = jira_client
        .get_components(jira_project_key.key)
        .await?
        .into_iter()
        .next()
        .map(|component| vec![ComponentId { id: component.id }])
        .unwrap_or_else(Vec::new);

    // Process creation of issues in parallel
    let mut issue_futures = FuturesUnordered::new();
    for _ in 0..qty {
        issue_futures.push(create_issue_task(
            jira_client.clone(),
            jira_project_key.clone(),
            first_component.clone(),
        ));
    }

    // Collect results
    let mut issues = Vec::new();
    while let Some(result) = issue_futures.next().await {
        match result {
            Ok(issue_key) => issues.push(issue_key),
            Err(e) => eprintln!("Task failed: {e}"),
        }
    }

    debug!("Successfully created {} issues", issues.len());
    Ok(issues)
}

#[allow(dead_code)]
pub async fn delete_batch_of_issues_by_key(issue_keys: &Vec<IssueKey>) {
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

#[allow(dead_code)] // This is a bug in the Rust linter, this function is called
pub async fn add_random_work_logs_to_issues(
    issues: &Vec<IssueKey>,
    worklog_qty_range: Range<i32>,
) -> Vec<Worklog> {
    let mut work_log_results = Vec::new();

    let mut work_log_futures = FuturesUnordered::new();

    for jira_key in issues {
        let jira_client = create_jira_client().await;

        debug!("Adding worklogs to issue {}", jira_key);

        work_log_futures.push(async move {
            let mut worklogs = Vec::new();
            for i in 0..worklog_qty_range.end {
                let worklog = jira_client
                    .insert_worklog(
                        jira_key.as_str(),
                        random_datetime(),
                        random_number_seconds_in_steps_of_900(),
                        "Test worklog",
                    )
                    .await;
                debug!("Added worklog {}/{}", i, worklog_qty_range.end);
                match worklog {
                    Ok(worklog) => {
                        assert!(!worklog.id.is_empty());
                        debug!("Added worklog {}", worklog.id);
                        worklogs.push(worklog);
                    }
                    Err(e) => {
                        eprintln!("Failed to add worklog, {}: ", e);
                        panic!("Failed to add worklog, {}: ", e);
                    }
                }
            }
            worklogs
        });

        while let Some(result) = work_log_futures.next().await {
            work_log_results.extend(result);
        }
    }
    work_log_results
}

#[allow(dead_code)]
pub fn random_datetime() -> DateTime<Local> {
    // Current datetime
    let now = Local::now();

    // 30 days ago
    let thirty_days_ago = now - Duration::days(30);

    // Total seconds in the range
    let total_seconds = (now - thirty_days_ago).num_seconds();

    // Generate a random number of seconds to subtract
    let random_seconds = thread_rng().gen_range(0..=total_seconds);

    // Subtract random seconds from now to generate a random datetime
    now - Duration::seconds(random_seconds)
}
#[allow(dead_code)]
pub fn random_number_seconds_in_steps_of_900() -> i32 {
    let min = 900;
    let max = 43200;
    let step = 900;

    // Calculate total steps in the range
    let step_count = (max - min) / step + 1;

    // Generate a random step index
    let random_step = thread_rng().gen_range(0..step_count);

    // Map the step index to the actual number
    min + random_step * step
}
