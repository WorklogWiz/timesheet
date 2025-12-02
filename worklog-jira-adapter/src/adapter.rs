//! Jira adapter implementation

use crate::conversions;
use async_trait::async_trait;
use jira::{Credentials, Jira};
use log::debug;
use std::collections::HashMap;
use std::sync::Arc;
use worklog_core::{
    traits::IssueTrackerClient, Issue, User, WorklogEntry, WorklogError, WorklogResult,
};

/// Jira implementation of `IssueTrackerClient`
///
/// This adapter bridges the generic worklog domain with Jira's specific API,
/// handling type conversions and API communication.
pub struct JiraAdapter {
    client: Arc<Jira>,
}

impl JiraAdapter {
    /// Create a new Jira adapter from a Jira client
    #[must_use]
    pub fn new(client: Jira) -> Self {
        Self {
            client: Arc::new(client),
        }
    }

    /// Create a new Jira adapter from configuration
    ///
    /// # Arguments
    /// * `base_url` - Jira base URL (e.g., "<https://your-domain.atlassian.net>")
    /// * `username` - Jira username/email
    /// * `token` - Jira API token
    ///
    /// # Errors
    /// Returns error if the Jira client cannot be created
    pub fn from_config(
        base_url: impl AsRef<str>,
        username: impl Into<String>,
        token: impl Into<String>,
    ) -> WorklogResult<Self> {
        let credentials = Credentials::Basic(username.into(), token.into());
        let client = Jira::new(base_url.as_ref(), credentials)
            .map_err(|e| WorklogError::ConfigError(format!("Failed to create Jira client: {e}")))?;

        Ok(Self::new(client))
    }

    /// Get a reference to the underlying Jira client
    ///
    /// This is useful for Jira-specific operations not covered by the generic trait.
    #[must_use]
    pub fn client(&self) -> &Jira {
        &self.client
    }

    /// Test Jira connection with credentials
    ///
    /// This method tests the connection without creating a full adapter instance.
    ///
    /// # Arguments
    /// * `base_url` - Jira base URL
    /// * `username` - Jira username/email
    /// * `token` - Jira API token
    ///
    /// # Returns
    /// The display name of the connected user on success
    ///
    /// # Errors
    /// Returns error if connection fails or credentials are invalid
    pub async fn test_connection(
        base_url: impl AsRef<str>,
        username: impl Into<String>,
        token: impl Into<String>,
    ) -> WorklogResult<String> {
        let credentials = Credentials::Basic(username.into(), token.into());
        let client = Jira::new(base_url.as_ref(), credentials)
            .map_err(|e| WorklogError::ConfigError(format!("Failed to create Jira client: {e}")))?;

        let user = client
            .get_current_user()
            .await
            .map_err(|e| WorklogError::AuthenticationError(format!("Connection failed: {e}")))?;

        Ok(user.display_name)
    }
}

#[async_trait]
impl IssueTrackerClient for JiraAdapter {
    fn from_url(url: &str, config: &HashMap<String, String>) -> WorklogResult<Self> {
        // Parse Jira URL (e.g., "jira://company.atlassian.net")
        if !url.starts_with("jira://") {
            return Err(WorklogError::ConfigError(format!(
                "Invalid Jira URL format: {url}. Expected format: jira://company.atlassian.net"
            )));
        }

        let base_url = url.strip_prefix("jira://").unwrap();

        // Extract required config
        let username = config.get("username").ok_or_else(|| {
            WorklogError::ConfigError("Missing 'username' in Jira config".to_string())
        })?;

        let token = config.get("token").ok_or_else(|| {
            WorklogError::ConfigError("Missing 'token' in Jira config".to_string())
        })?;

        // Build full HTTPS URL
        let full_url = if base_url.starts_with("http://") || base_url.starts_with("https://") {
            base_url.to_string()
        } else {
            format!("https://{base_url}")
        };

        // Create Jira client
        let credentials = Credentials::Basic(username.clone(), token.clone());
        let client = Jira::new(&full_url, credentials)
            .map_err(|e| WorklogError::ConfigError(format!("Failed to create Jira client: {e}")))?;

        Ok(Self::new(client))
    }

    fn tracker_name(&self) -> &'static str {
        "Jira"
    }

    async fn get_issue(&self, key: &str) -> WorklogResult<Issue> {
        debug!("Fetching Jira issue: {key}");

        let issue_key = jira::models::core::IssueKey::new(key);
        let jira_issue = self
            .client
            .get_issue_summary(&issue_key)
            .await
            .map_err(|e| {
                WorklogError::IssueTrackerError(format!("Failed to fetch issue {key}: {e}"))
            })?;

        Ok(conversions::issue_from_jira(&jira_issue))
    }

    async fn search_issues(&self, query: &str, limit: usize) -> WorklogResult<Vec<Issue>> {
        debug!("Searching Jira with JQL: {query}");

        // For Jira, the query is JQL (Jira Query Language)
        let jira_issues = self
            .client
            .fetch_with_jql(query, vec!["key", "summary"])
            .await
            .map_err(|e| {
                WorklogError::IssueTrackerError(format!("Failed to search issues: {e}"))
            })?;

        // Take only the requested limit
        let issues: Vec<Issue> = jira_issues
            .iter()
            .take(limit)
            .map(conversions::issue_from_jira)
            .collect();

        debug!("Found {} issues", issues.len());
        Ok(issues)
    }

    async fn sync_worklog(&self, entry: &WorklogEntry) -> WorklogResult<String> {
        debug!("Syncing worklog entry to Jira: {:?}", entry.id);

        // Validation
        if entry.is_active() {
            return Err(WorklogError::CannotSync(
                "Cannot sync active worklog entry (timer still running)".to_string(),
            ));
        }

        let issue_key = entry.issue_key.as_ref().ok_or_else(|| {
            WorklogError::CannotSync("Worklog entry has no issue_key".to_string())
        })?;

        let time_spent_seconds = entry.time_spent_seconds.ok_or_else(|| {
            WorklogError::CannotSync("Worklog entry has no time_spent_seconds".to_string())
        })?;

        // Create worklog in Jira
        let jira_worklog = self
            .client
            .insert_worklog(
                issue_key,
                entry.started_at,
                time_spent_seconds,
                entry.comment.as_deref().unwrap_or(""),
            )
            .await
            .map_err(|e| {
                WorklogError::IssueTrackerError(format!("Failed to create worklog in Jira: {e}"))
            })?;

        debug!("Created Jira worklog: {}", jira_worklog.id);
        Ok(jira_worklog.id)
    }

    async fn get_current_user(&self) -> WorklogResult<User> {
        debug!("Fetching current Jira user");

        let jira_user = self.client.get_current_user().await.map_err(|e| {
            WorklogError::AuthenticationError(format!("Failed to fetch current user: {e}"))
        })?;

        Ok(conversions::user_from_jira(jira_user))
    }

    async fn check_connection(&self) -> WorklogResult<bool> {
        debug!("Checking Jira connection");

        match self.client.get_current_user().await {
            Ok(_) => {
                debug!("Jira connection successful");
                Ok(true)
            }
            Err(e) => Err(WorklogError::IssueTrackerError(format!(
                "Jira connection failed: {e}"
            ))),
        }
    }

    async fn search_issues_in_project(
        &self,
        project_key: &str,
        all_users: Option<bool>,
    ) -> WorklogResult<Vec<Issue>> {
        debug!("Searching issues in project: {project_key} (all_users filter: {all_users:?})");

        let jira_issues = self
            .client
            .get_issue_summaries(&[project_key], &[], all_users)
            .await
            .map_err(|e| {
                WorklogError::IssueTrackerError(format!("Failed to search issues: {e}"))
            })?;

        Ok(jira_issues
            .iter()
            .map(conversions::issue_from_jira)
            .collect())
    }

    async fn get_worklogs_for_issue(
        &self,
        issue_key: &str,
        since: chrono::DateTime<chrono::Utc>,
    ) -> WorklogResult<Vec<WorklogEntry>> {
        debug!("Fetching worklogs for {issue_key} since {since}");

        // Convert UTC to Local for Jira client
        let since_local = chrono::DateTime::<chrono::Local>::from(since);

        // Get worklogs from Jira (filtered for current user)
        let jira_worklogs = self
            .client
            .get_work_logs_for_current_user(issue_key, Some(since_local))
            .await
            .map_err(|e| {
                WorklogError::IssueTrackerError(format!("Failed to fetch worklogs: {e}"))
            })?;

        debug!("Retrieved {} worklogs from Jira", jira_worklogs.len());

        // Convert Jira worklogs to WorklogEntry format
        let mut entries = Vec::new();
        for jira_wl in jira_worklogs {
            let started_local = jira_wl.started.with_timezone(&chrono::Local);
            let stopped_local = (jira_wl.started
                + chrono::Duration::seconds(i64::from(jira_wl.timeSpentSeconds)))
            .with_timezone(&chrono::Local);

            let created_local = jira_wl.created.with_timezone(&chrono::Local);
            let updated_local = jira_wl.updated.with_timezone(&chrono::Local);
            let now = chrono::Local::now();

            let entry = WorklogEntry {
                id: None, // Will be assigned by local DB
                issue_key: Some(issue_key.to_string()),
                started_at: started_local,
                stopped_at: Some(stopped_local),
                comment: jira_wl.comment,
                tags: vec![],
                time_spent_seconds: Some(jira_wl.timeSpentSeconds),
                synced_to_provider: true,
                provider_worklog_id: Some(jira_wl.id),
                created_at: created_local,
                updated_at: updated_local, // Use Jira's updated timestamp
                deleted_at: None,
                last_synced_at: Some(now), // Just synced from Jira
                version: 1,
                has_conflict: false,
            };
            entries.push(entry);
        }

        debug!("Converted to {} WorklogEntry objects", entries.len());
        Ok(entries)
    }

    async fn delete_worklog(&self, issue_key: &str, worklog_id: &str) -> WorklogResult<()> {
        debug!("Deleting worklog {worklog_id} from issue {issue_key}");

        self.client
            .delete_worklog(issue_key.to_string(), worklog_id.to_string())
            .await
            .map_err(|e| {
                WorklogError::IssueTrackerError(format!("Failed to delete worklog: {e}"))
            })?;

        Ok(())
    }

    async fn get_recently_worked_issues(&self) -> WorklogResult<Vec<Issue>> {
        let jql = "worklogAuthor = currentUser() AND worklogDate >= -180d ORDER BY updated DESC";

        let jira_issues = self
            .client
            .fetch_with_jql::<jira::models::issue::IssueSummary>(jql, vec!["key", "summary"])
            .await
            .map_err(|e| WorklogError::IssueTrackerError(format!("JQL query failed: {e}")))?;

        // Convert Jira IssueSummary to worklog_core::Issue
        let issues = jira_issues
            .into_iter()
            .map(|summary| conversions::issue_from_jira(&summary))
            .collect();

        Ok(issues)
    }
}

#[cfg(test)]
mod tests {
    // Note: Comprehensive testing requires a real Jira instance or mocking the HTTP layer.
    // The conversions module has unit tests that don't require external dependencies.
    // Integration tests would go in tests/ directory with appropriate test infrastructure.
}
