// worklog/tests/timer_service_test.rs

use chrono::{Duration, Local, Utc};
use jira::models::core::Fields;
use jira::models::issue::IssueSummary;
use jira::models::project::JiraProjectKey;
use log::debug;

// Optional: import helper modules
mod test_helpers;
use crate::test_helpers::fixtures::{create_test_issues, create_test_timer};
use test_helpers::common::create_test_runtime;
use test_helpers::fixtures::TEST_ISSUE_KEY;
#[test]
fn test_start_and_stop_timer() {
    // Initialize logger only once across tests
    let _ = env_logger::builder().is_test(true).try_init();

    // Set up a test runtime with a temporary database
    let runtime = create_test_runtime().expect("Failed to create test runtime");

    let test_issues = create_test_issues();
    runtime
        .issue_service
        .add_jira_issues(&test_issues)
        .expect("Failed to add test issues");

    // Act: Start a timer
    let timer = runtime
        .timer_service()
        .start_timer(TEST_ISSUE_KEY, Some("Test work".to_string()))
        .expect("Failed to start timer");

    // Assert: Timer was created correctly
    assert_eq!(timer.issue_key, TEST_ISSUE_KEY);
    assert!(timer.is_active());

    // Act: Stop the timer
    let stopped_timer = runtime
        .timer_service()
        .stop_active_timer(Some(Utc::now().with_timezone(&Local) + Duration::hours(3)))
        .expect("Failed to stop timer");

    // Assert: Timer was stopped
    assert!(!stopped_timer.is_active());
    assert!(stopped_timer.stopped_at.is_some());
}

#[tokio::test]
async fn test_sync_timers_to_jira() {
    // Initialize logger only once across tests
    let _ = env_logger::builder().is_test(true).try_init();

    // Set up a test runtime
    let runtime = create_test_runtime().expect("Failed to create test runtime");

    // Create a test issue in Jira
    let test_issue = runtime
        .jira_client()
        .create_issue(
            &JiraProjectKey { key: "TWIZ" },
            "TEST summary",
            None,
            vec![],
        )
        .await
        .expect("Failed to create test issue");
    debug!("Created Jira issue {test_issue:?}");

    // Retrieves the issue summaries from Jira
    let issue_key_clone = test_issue.key.clone();

    let issue_summary = IssueSummary {
        id: test_issue.id,
        key: issue_key_clone,
        fields: Fields {
            summary: "TEST Summary".to_string(),
            components: vec![],
        },
    };

    // .. and inserts them into the database
    runtime
        .issue_service()
        .add_jira_issues(&[issue_summary])
        .expect("Failed to add test issues");

    // Insert the test timers directly into the database for testing
    let timer_service = runtime.timer_service();
    let test_timer = create_test_timer(test_issue.key.value(), true);

    let result = timer_service
        .start_timer(&test_timer.issue_key, Some("Rubbish".to_string()))
        .expect("Failed to start test timer ");

    assert_eq!(result.issue_key, test_timer.issue_key);
    let stop_time = Utc::now().with_timezone(&Local) + Duration::hours(3);
    let timer = timer_service
        .stop_active_timer(Some(stop_time))
        .expect("Failed to stop test timer");

    assert_eq!(result.issue_key, timer.issue_key);
    assert!(timer.stopped_at.is_some());
    assert!(timer.duration().is_some());
    debug!("Timer duration: {}", timer.duration().unwrap());

    let _result = timer_service
        .sync_timers_to_jira()
        .await
        .expect("Failed to sync timers to Jira");

    runtime
        .client
        .delete_issue(&test_issue.key)
        .await
        .expect("Failed to delete test issue");
}
