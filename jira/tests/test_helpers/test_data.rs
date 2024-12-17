use crate::test_helpers::jira_client::create_jira_client;
use chrono::{DateTime, Duration, Local};
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use jira::models::core::JiraKey;
use jira::models::project::JiraProjectKey;
// Ensure JiraProjectKey is defined to derive Clone
use jira::models::worklog::Worklog;
use log::debug;
use rand::{thread_rng, Rng};
use std::ops::Range;

#[allow(dead_code)]
pub async fn create_batch_of_issues(
    qty: i32,
    jira_project_key: JiraProjectKey,
) -> Result<Vec<JiraKey>, Box<dyn std::error::Error>> {
    let jira_client = create_jira_client().await;

    // Create a stream of futures to process in parallel
    let mut issue_futures = FuturesUnordered::new();

    let start_time = std::time::Instant::now();
    for _ in 0..qty {
        let jira_client_clone = jira_client.clone(); // Clone the client to use in each task
        let project_key = jira_project_key.clone(); // Move to capture project_key behavior.
                                                    // Task must return a Result
        issue_futures.push(async move {
            let new_issue = jira_client_clone
                .create_issue(
                    &project_key,
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

#[allow(dead_code)]
pub async fn delete_batch_of_issues_by_key(issue_keys: &Vec<JiraKey>) {
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
    issues: &Vec<JiraKey>,
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
