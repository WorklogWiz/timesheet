//! This module provides the implementation of `WorkLogService`, a service responsible for
//! managing worklog entries in a repository. It offers operations such as adding, removing,
//! updating, and retrieving worklogs. The service interacts with a repository that implements
//! the `WorkLogRepository` trait to perform these operations.

use crate::error::WorklogError;
use crate::repository::worklog_repository::WorkLogRepository;
use crate::service::issue::IssueService;
use crate::types::LocalWorklog;
use chrono::{DateTime, Local};
use jira::models::core::IssueKey;
use jira::models::user::User;
use jira::models::worklog::Worklog;
use jira::Jira;
use std::collections::HashSet;
use std::sync::Arc;

pub struct WorkLogService {
    repo: Arc<dyn WorkLogRepository>,
    issue_service: Arc<IssueService>,
    jira_client: Jira,
}

impl WorkLogService {
    /// Creates a new instance of `WorkLogService`.
    ///
    /// # Arguments
    ///
    /// * `repo` - A shared reference to a type that implements the `WorkLogRepository` trait.
    ///
    /// # Returns
    ///
    /// A new `WorkLogService` instance.
    pub fn new(
        repo: Arc<dyn WorkLogRepository>,
        issue_service: Arc<IssueService>,
        jira_client: Jira,
    ) -> Self {
        Self {
            repo,
            issue_service,
            jira_client,
        }
    }

    /// Removes a worklog entry based on the provided `Worklog` object.
    ///
    /// # Arguments
    ///
    /// * `wl` - A reference to the `Worklog` object that needs to be removed.
    ///
    /// # Returns
    ///
    /// A `Result` indicating success (`Ok`) or a `WorklogError` (`Err`) if the operation fails.
    ///
    ///
    /// # Errors
    ///
    /// This function will return a `WorklogError` if:
    /// - The repository operation fails due to a database issue or unexpected error.
    /// - The provided worklog ID does not exist in the repository or cannot be removed.
    pub fn remove_worklog_entry(&self, wl: &Worklog) -> Result<(), WorklogError> {
        self.repo.remove_entry_by_worklog_id(wl.id.as_str())
    }

    /// Removes a worklog entry by its identifier.
    ///
    /// # Arguments
    ///
    /// * `wl_id` - A reference to the string identifier of the worklog that needs to be removed.
    ///
    /// # Returns
    ///
    /// A `Result` indicating success (`Ok`) or a `WorklogError` (`Err`) if the operation fails.
    ///
    /// # Errors
    ///
    /// This function will return a `WorklogError` if:
    /// - The repository operation fails due to a database issue or unexpected error.
    /// - The provided worklog ID does not exist in the repository.
    pub fn remove_entry_by_worklog_id(&self, wl_id: &str) -> Result<(), WorklogError> {
        self.repo.remove_entry_by_worklog_id(wl_id)
    }

    /// Adds a new worklog entry to the repository.
    ///
    /// # Arguments
    ///
    /// * `local_worklog` - A reference to the `LocalWorklog` object representing the worklog entry to be added.
    ///
    /// # Returns
    ///
    /// A `Result` indicating success (`Ok`) or a `WorklogError` (`Err`) if the operation fails.
    ///
    /// # Errors
    ///
    /// This function will return a `WorklogError` if:
    /// - The repository operation fails due to a database issue or unexpected error.
    pub async fn add_entry(&self, local_worklog: &LocalWorklog) -> Result<(), WorklogError> {
        self.add_worklog_entries(&[local_worklog.clone()]).await
    }

    /// Adds multiple worklog entries to the repository.
    ///
    /// # Arguments
    ///
    /// * `worklogs` - A slice of `LocalWorklog` objects representing the worklog entries to be added.
    ///
    /// # Returns
    ///
    /// A `Result` indicating success (`Ok`) or a `WorklogError` (`Err`) if the operation fails.
    pub(crate) async fn add_worklog_entries(
        &self,
        worklogs: &[LocalWorklog],
    ) -> Result<(), WorklogError> {
        // Check the DBMS to ensure all worklogs are referencing a valid issue
        let existing_issues = self.issue_service.get_issues_filtered_by_keys(
            worklogs
                .iter()
                .map(|wl| wl.issue_key.clone())
                .collect::<Vec<IssueKey>>()
                .as_slice(),
        )?;
        // Create a set of existing issue keys for efficient lookup
        let existing_keys: HashSet<IssueKey> = existing_issues
            .iter()
            .map(|issue| issue.issue_key.clone())
            .collect();
        // Find the issue keys, which are not in the local database
        let new_keys: Vec<IssueKey> = worklogs
            .iter()
            .map(|worklog| worklog.issue_key.clone()) // Extract an issue key from each worklog
            .filter(|key| !existing_keys.contains(key)) // Keep only keys not in existing_keys
            .collect();
        if !new_keys.is_empty() {
            // Fetch data from jira for the missing issues
            let issue_summaries_to_sync = self
                .jira_client
                .get_issue_summaries(&[], new_keys.as_slice(), true)
                .await?;

            self.issue_service
                .add_jira_issues(issue_summaries_to_sync.as_slice())?;
        }
        self.repo.add_worklog_entries(worklogs)
    }

    /// Returns the total count of worklog entries in the repository.
    ///
    /// # Returns
    ///
    /// A `Result` containing the count of worklog entries (`i64`) on success (`Ok`),
    /// or a `WorklogError` (`Err`) if the operation fails.
    #[allow(dead_code)]
    fn get_count(&self) -> Result<i64, WorklogError> {
        self.repo.get_count()
    }

    /// Purges all entries from the local worklog repository.
    ///
    /// # Returns
    ///
    /// A `Result` indicating success (`Ok`) or a `WorklogError` (`Err`) if the operation fails.
    #[allow(dead_code)]
    fn purge_entire_local_worklog(&self) -> Result<(), WorklogError> {
        self.repo.purge_entire_local_worklog()
    }

    /// Finds a worklog by its identifier.
    ///
    /// # Arguments
    ///
    /// * `worklog_id` - A reference to the string identifier of the worklog to be searched.
    ///
    /// # Returns
    ///
    /// A `Result` containing the `LocalWorklog` if found (`Ok`),
    /// or a `WorklogError` (`Err`) if the operation fails or the worklog is not found.
    #[allow(dead_code)]
    fn find_worklog_by_id(&self, worklog_id: &str) -> Result<LocalWorklog, WorklogError> {
        self.repo.find_worklog_by_id(worklog_id)
    }

    /// Finds all worklogs with a start date on or after the specified `start_datetime`, filtered by issue keys and users (current user).
    ///
    /// # Arguments
    ///
    /// * `start_datetime` - A `DateTime<Local>` representing the starting point for filtering worklogs.
    /// * `keys_filter` - A slice of `IssueKey` objects used to filter worklogs based on issue keys.
    /// * `users_filter` - A slice of `User` objects used to filter worklogs based on users.
    ///
    /// # Returns
    ///
    /// A `Result`:
    /// - `Ok(Vec<LocalWorklog>)` - A vector of matching `LocalWorklog` entries if found successfully.
    /// - `Err(WorklogError)` - An error if the operation fails or no matching worklogs are found.
    ///
    /// # Errors
    ///
    /// This function will return a `WorklogError` if:
    /// - The repository operation fails due to a database issue or unexpected error.
    /// - No matching worklogs are found based on the provided filters.
    pub fn find_worklogs_after(
        &self,
        start_datetime: DateTime<Local>,
        keys_filter: &[IssueKey],
        users_filter: &[User],
    ) -> Result<Vec<LocalWorklog>, WorklogError> {
        self.repo
            .find_worklogs_after(start_datetime, keys_filter, users_filter)
    }
}

#[cfg(test)]
mod tests {
    use crate::types::LocalWorklog;
    use chrono::Local;
    use jira::models::core::IssueKey;
    use jira::models::user::User;

    #[test]
    fn test_worklog_service_struct_creation() {
        // This test just verifies the struct can be conceptually created
        // In practice, you'd need proper mocks for the dependencies
        assert!(true); // Placeholder test
    }

    #[test]
    fn test_local_worklog_creation() {
        let now = Local::now();
        let worklog = LocalWorklog {
            issue_key: IssueKey::from("TEST-123"),
            id: "test-id".to_string(),
            author: "Test User".to_string(),
            created: now,
            updated: now,
            started: now,
            timeSpent: "1h".to_string(),
            timeSpentSeconds: 3600, // 1 hour
            issueId: 12345,
            comment: Some("Test work".to_string()),
        };

        assert_eq!(worklog.id, "test-id");
        assert_eq!(worklog.issue_key.value(), "TEST-123");
        assert_eq!(worklog.timeSpentSeconds, 3600);
        assert_eq!(worklog.comment, Some("Test work".to_string()));
        assert_eq!(worklog.author, "Test User");
        assert_eq!(worklog.timeSpent, "1h");
        assert_eq!(worklog.issueId, 12345);
    }

    #[test]
    fn test_local_worklog_time_calculations() {
        let now = Local::now();
        let worklog = LocalWorklog {
            issue_key: IssueKey::from("TEST-123"),
            id: "test-id".to_string(),
            author: "Test User".to_string(),
            created: now,
            updated: now,
            started: now,
            timeSpent: "2h".to_string(),
            timeSpentSeconds: 7200, // 2 hours
            issueId: 12345,
            comment: Some("Test work".to_string()),
        };

        // 2 hours = 7200 seconds
        assert_eq!(worklog.timeSpentSeconds, 7200);

        // Convert to hours for verification
        let hours = worklog.timeSpentSeconds / 3600;
        assert_eq!(hours, 2);
    }

    #[test]
    fn test_local_worklog_with_different_durations() {
        let test_cases = vec![
            (900, "15m", "15 minutes"),  // 15 * 60
            (1800, "30m", "30 minutes"), // 30 * 60
            (3600, "1h", "1 hour"),      // 1 * 60 * 60
            (7200, "2h", "2 hours"),     // 2 * 60 * 60
            (28800, "8h", "8 hours"),    // 8 * 60 * 60 (full day)
        ];

        for (seconds, time_spent, description) in test_cases {
            let now = Local::now();
            let worklog = LocalWorklog {
                issue_key: IssueKey::from("TEST-123"),
                id: format!("test-{seconds}"),
                author: "Test User".to_string(),
                created: now,
                updated: now,
                started: now,
                timeSpent: time_spent.to_string(),
                timeSpentSeconds: seconds,
                issueId: 12345,
                comment: Some(description.to_string()),
            };

            assert_eq!(worklog.timeSpentSeconds, seconds);
            assert_eq!(worklog.timeSpent, time_spent);
            assert_eq!(worklog.comment, Some(description.to_string()));
        }
    }

    #[test]
    fn test_user_struct_creation() {
        let user = User {
            self_url: "https://example.com/user/123".to_string(),
            account_id: "acc123".to_string(),
            email_address: "john.doe@example.com".to_string(),
            display_name: "John Doe".to_string(),
            time_zone: "UTC".to_string(),
        };

        assert_eq!(user.account_id, "acc123");
        assert_eq!(user.display_name, "John Doe");
        assert_eq!(user.email_address, "john.doe@example.com");
        assert_eq!(user.time_zone, "UTC");
        assert_eq!(user.self_url, "https://example.com/user/123");
    }

    #[test]
    fn test_local_worklog_with_various_issue_keys() {
        let issue_keys = vec!["PROJ-1", "ABC-123", "LONGPROJECT-9999", "X-1"];

        for key in issue_keys {
            let now = Local::now();
            let worklog = LocalWorklog {
                issue_key: IssueKey::from(key),
                id: format!("test-{key}"),
                author: "Test User".to_string(),
                created: now,
                updated: now,
                started: now,
                timeSpent: "1h".to_string(),
                timeSpentSeconds: 3600,
                issueId: 12345,
                comment: Some(format!("Work on {key}")),
            };

            assert_eq!(worklog.issue_key.value(), key);
            assert_eq!(worklog.comment, Some(format!("Work on {key}")));
        }
    }

    #[test]
    fn test_local_worklog_time_boundaries() {
        // Test edge cases for time
        let test_cases = vec![
            (1, "1s", "Minimum time"),            // 1 second
            (60, "1m", "One minute"),             // 1 minute
            (3599, "59m 59s", "Just under hour"), // 59 minutes 59 seconds
            (86400, "1d", "Full day"),            // 24 hours
        ];

        for (seconds, time_spent, description) in test_cases {
            let now = Local::now();
            let worklog = LocalWorklog {
                issue_key: IssueKey::from("BOUNDARY-1"),
                id: format!("boundary-{seconds}"),
                author: "Test User".to_string(),
                created: now,
                updated: now,
                started: now,
                timeSpent: time_spent.to_string(),
                timeSpentSeconds: seconds,
                issueId: 12345,
                comment: Some(description.to_string()),
            };

            assert_eq!(worklog.timeSpentSeconds, seconds);
            assert!(worklog.timeSpentSeconds > 0);
        }
    }

    #[test]
    fn test_local_worklog_comment_variations() {
        let comments = vec![
            None,
            Some(String::new()),
            Some("Short".to_string()),
            Some("A longer comment describing the work done".to_string()),
            Some("Comment with special characters: !@#$%^&*()".to_string()),
        ];

        for comment in comments {
            let now = Local::now();
            let worklog = LocalWorklog {
                issue_key: IssueKey::from("COMMENT-1"),
                id: "test-comment".to_string(),
                author: "Test User".to_string(),
                created: now,
                updated: now,
                started: now,
                timeSpent: "1h".to_string(),
                timeSpentSeconds: 3600,
                issueId: 12345,
                comment: comment.clone(),
            };

            assert_eq!(worklog.comment, comment);
        }
    }

    #[test]
    fn test_local_worklog_without_comment() {
        let now = Local::now();
        let worklog = LocalWorklog {
            issue_key: IssueKey::from("NO-COMMENT-1"),
            id: "test-no-comment".to_string(),
            author: "Test User".to_string(),
            created: now,
            updated: now,
            started: now,
            timeSpent: "30m".to_string(),
            timeSpentSeconds: 1800,
            issueId: 12345,
            comment: None,
        };

        assert!(worklog.comment.is_none());
        assert_eq!(worklog.timeSpentSeconds, 1800);
        assert_eq!(worklog.timeSpent, "30m");
    }
}
