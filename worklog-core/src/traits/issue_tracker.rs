//! Issue tracker client trait

use crate::domain::{Issue, User, WorklogEntry};
use crate::error::WorklogResult;
use async_trait::async_trait;
use std::collections::HashMap;

#[cfg(test)]
use mockall::automock;

/// Generic interface for issue tracking systems
///
/// This trait abstracts over different issue trackers (Jira, GitHub, Linear, etc.),
/// providing a consistent interface for the worklog system.
///
/// # Connection URL Format
///
/// The `url` parameter follows these patterns:
/// - Jira: `jira://company.atlassian.net` or `jira://jira.company.com` (self-hosted)
/// - GitHub: `github://api.github.com` or `github://github.company.com` (enterprise)
/// - Linear: `linear://api.linear.app`
/// - Azure DevOps: `azdo://dev.azure.com/organization`
/// - GitLab: `gitlab://gitlab.com` or `gitlab://gitlab.company.com` (self-hosted)
///
/// # Configuration
///
/// Provider-specific authentication and context passed as key-value pairs:
///
/// **Jira:**
/// - `username`: User email
/// - `token`: API token
/// - `project`: (Optional) Default project key
///
/// **GitHub:**
/// - `token`: Personal access token
/// - `owner`: Repository owner
/// - `repo`: Repository name
///
/// **Linear:**
/// - `api_key`: Linear API key
/// - `team_id`: (Optional) Team identifier
///
/// # Design Philosophy
///
/// - **Provider Agnostic**: Methods work with any issue tracker
/// - **Minimal Surface**: Only operations needed by worklog
/// - **Error Tolerant**: Returns `Result` for all fallible operations
/// - **Secure**: Credentials in config, not URLs
///
/// # Example
///
/// ```ignore
/// use worklog_core::traits::IssueTrackerClient;
/// use std::collections::HashMap;
///
/// let mut config = HashMap::new();
/// config.insert("username".to_string(), "user@company.com".to_string());
/// config.insert("token".to_string(), "secret_token".to_string());
///
/// let client = JiraClient::from_url("jira://company.atlassian.net", &config)?;
/// ```
#[cfg_attr(test, automock)]
#[async_trait]
pub trait IssueTrackerClient: Send + Sync {
    /// Create a new issue tracker client from a URL and configuration
    ///
    /// # Arguments
    ///
    /// * `url` - Connection URL (format depends on tracker provider)
    /// * `config` - Provider-specific authentication and settings
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - URL format is invalid
    /// - Required config keys are missing
    /// - Authentication fails
    /// - Connection fails
    fn from_url(url: &str, config: &HashMap<String, String>) -> WorklogResult<Self>
    where
        Self: Sized;

    /// Get the name of this issue tracker (e.g., "Jira", "GitHub", "Linear")
    ///
    /// Used for logging and user-facing messages.
    fn tracker_name(&self) -> &'static str;
    /// Get details about a specific issue
    ///
    /// # Arguments
    /// * `key` - The issue key/identifier (e.g., "PROJ-123", "#456")
    ///
    /// # Returns
    /// Returns the issue details or an error if not found
    async fn get_issue(&self, key: &str) -> WorklogResult<Issue>;

    /// Search for issues matching a query
    ///
    /// The query format is provider-specific:
    /// - Jira: JQL (e.g., "project = PROJ AND status = Open")
    /// - GitHub: GitHub search syntax
    /// - Linear: Linear query syntax
    ///
    /// # Arguments
    /// * `query` - Search query (provider-specific format)
    /// * `limit` - Maximum number of results
    ///
    /// # Returns
    /// Vector of matching issues, may be empty
    async fn search_issues(&self, query: &str, limit: usize) -> WorklogResult<Vec<Issue>>;

    /// Sync a completed worklog entry to the issue tracker
    ///
    /// This creates a work log/time entry on the issue in the external system.
    ///
    /// # Arguments
    /// * `entry` - The completed worklog entry to sync
    ///
    /// # Returns
    /// Returns the provider's worklog ID (e.g., Jira worklog ID)
    ///
    /// # Errors
    /// Returns error if:
    /// - Entry is still active (not stopped)
    /// - Entry has no `issue_key`
    /// - Issue doesn't exist in tracker
    /// - Authentication fails
    /// - Network error
    async fn sync_worklog(&self, entry: &WorklogEntry) -> WorklogResult<String>;

    /// Get information about the current authenticated user
    ///
    /// # Returns
    /// User information or error if not authenticated
    async fn get_current_user(&self) -> WorklogResult<User>;

    /// Check if the client is properly configured and can connect
    ///
    /// This is useful for validation during setup.
    ///
    /// # Returns
    /// `Ok(true)` if connection successful, error otherwise
    async fn check_connection(&self) -> WorklogResult<bool>;

    /// Search for issues in a specific project
    ///
    /// # Arguments
    /// * `project_key` - The project key/identifier (e.g., "PROJ", "myrepo")
    /// * `all_users` - If Some(true), include issues from all users with worklogs;
    ///                 if Some(false), only issues where current user has worklogs;
    ///                 if None, return ALL issues regardless of worklogs
    ///
    /// # Returns
    /// Vector of issues in the project
    async fn search_issues_in_project(
        &self,
        project_key: &str,
        all_users: Option<bool>,
    ) -> WorklogResult<Vec<Issue>>;

    /// Get worklog entries for a specific issue
    ///
    /// # Arguments
    /// * `issue_key` - The issue key/identifier
    /// * `since` - Only return worklogs created/updated after this time
    ///
    /// # Returns
    /// Vector of worklog entries for the issue
    async fn get_worklogs_for_issue(
        &self,
        issue_key: &str,
        since: chrono::DateTime<chrono::Utc>,
    ) -> WorklogResult<Vec<WorklogEntry>>;

    /// Delete a worklog entry from the issue tracker
    ///
    /// # Arguments
    /// * `issue_key` - The issue key/identifier
    /// * `worklog_id` - The provider's worklog ID
    ///
    /// # Returns
    /// Ok(()) if successful
    async fn delete_worklog(&self, issue_key: &str, worklog_id: &str) -> WorklogResult<()>;

    /// Get issues the current user has recently worked on
    ///
    /// Returns issues where the current user has logged work time recently
    /// (typically last 180 days, but provider-specific).
    ///
    /// # Returns
    /// Vector of issues with recent worklog activity from the current user
    async fn get_recently_worked_issues(&self) -> WorklogResult<Vec<Issue>>;
}
