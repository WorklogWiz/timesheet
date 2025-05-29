#[cfg(test)]
#[allow(dead_code)]
mod test_helpers;

use crate::test_helpers::common::{create_test_runtime, TEST_PROJECT_KEY};
use crate::test_helpers::issue_tracker::IssueTracker;
use crate::test_helpers::test_cleanup::TestCleanup;
use jira::models::core::IssueKey;
use jira::models::project::JiraProjectKey;
use std::sync::Arc;
use worklog::error::WorklogError;
use worklog::operation::add;
use worklog::{ApplicationRuntime, ApplicationRuntimeBuilder};

struct WorkLogServiceTestContext {
    runtime: Arc<ApplicationRuntime>,
    issue_tracker: IssueTracker,
}

impl WorkLogServiceTestContext {
    fn new() -> Self {
        // Initialize logger only once
        let _ = env_logger::builder().is_test(true).try_init();

        Self {
            runtime: create_test_runtime().expect("Failed to create test runtime"),
            issue_tracker: IssueTracker::new(),
        }
    }

    #[allow(dead_code)]
    fn runtime(&self) -> &ApplicationRuntime {
        &self.runtime
    }
    #[allow(dead_code)]
    fn last_jira_key(&self) -> Option<IssueKey> {
        self.issue_tracker.last_key()
    }

    async fn with_new_jira_issue(&mut self) -> IssueKey {
        let new_issue_response = self
            .runtime
            .jira_client
            .create_issue(
                &JiraProjectKey {
                    key: TEST_PROJECT_KEY,
                },
                "Testing summary",
                Some("Test description".into()),
                vec![],
            )
            .await
            .expect("Failed to create issue");
        self.issue_tracker.track(new_issue_response.key.clone());
        new_issue_response.key
    }
}

#[async_trait::async_trait]
impl TestCleanup for WorkLogServiceTestContext {
    async fn cleanup(&mut self) {
        self.issue_tracker.cleanup(&self.runtime.jira_client).await;
    }
}

impl Drop for WorkLogServiceTestContext {
    fn drop(&mut self) {
        assert!(
            self.issue_tracker.is_clean(),
            "Issue tracker is not clean. Did you forget to call cleanup()?"
        );
    }
}
#[tokio::test]
async fn test_add_worklog_to_issue_not_synchronized() {
    let mut ctx = WorkLogServiceTestContext::new();
    let key = ctx.with_new_jira_issue().await;

    let mut add_params = add::Add {
        durations: vec!["1h".to_string()],
        issue_key: key.to_string(),
        started: None,
        comment: Some("Rubbish".to_string()),
    };

    let add_result = worklog::operation::add::execute(&ctx.runtime, &mut add_params).await;

    ctx.cleanup().await; // Make sure we clean up before we check the results and fail the test
    assert!(
        add_result.is_ok(),
        "Failed to add worklog: {}",
        add_result.unwrap_err()
    );
}

#[tokio::test]
async fn test_add_to_empty_issue_not_synchronized() {
    let mut ctx = WorkLogServiceTestContext::new();

    let mut add_params = add::Add {
        durations: vec!["1h".to_string()],
        issue_key: "TWIZ-1".to_string(),
        started: None,
        comment: Some("Rubbish".to_string()),
    };

    let add_result = worklog::operation::add::execute(&ctx.runtime, &mut add_params).await;

    ctx.cleanup().await; // Make sure we clean up before we check the results and fail the test
    assert!(
        add_result.is_ok(),
        "Failed to add worklog: {}",
        add_result.unwrap_err()
    );
}

#[allow(dead_code)]
fn assert_send_sync<T: Send + Sync>(_: T) {}

/// Ensures that the `ApplicationRuntime` instance created using the builder
/// is properly configured for concurrent usage and can support threading
/// by implementing the `Send` and `Sync` traits.
///
/// This test creates an in-memory runtime for testing purposes,
/// avoiding file I/O while maintaining logical integrity of the runtime's services.
///
/// # Usage
///
/// Run the test using:
///
/// ```bash
/// cargo test test_create_in_memory_runtime
/// ```
///
/// # Assertions
///
/// - The `ApplicationRuntime` instance must successfully initialize.
/// - The runtime instance must implement `Send` and `Sync` traits.
///
/// # Errors
///
/// If the configuration cannot be loaded or any of the runtime's dependencies
/// fail to initialize, the test will panic.
#[test]
pub fn test_create_in_memory_runtime() -> Result<(), WorklogError> {
    let runtime = ApplicationRuntimeBuilder::default()
        .use_jira_test_instance()
        .use_in_memory_db()
        .build()?;
    assert_send_sync(runtime);
    Ok(())
}
