//! Service for managing issues from any issue tracker
//!
//! The `IssueService` provides functionalities for interacting with a local database of issues
//! and fetching fresh data from the remote issue tracker.
//! It works with any issue tracker (Jira, GitHub, Linear, etc.) through the generic `Issue` type.

use crate::domain::Issue;
use crate::error::WorklogError;
use crate::traits::{IssueRepository, IssueTrackerClient};
use std::sync::Arc;

#[allow(clippy::module_name_repetitions)]
pub struct IssueService {
    repo: Arc<dyn IssueRepository>,
    tracker: Arc<dyn IssueTrackerClient>,
}

#[allow(clippy::module_name_repetitions)]
#[allow(clippy::missing_errors_doc)]
impl IssueService {
    pub fn new(repo: Arc<dyn IssueRepository>, tracker: Arc<dyn IssueTrackerClient>) -> Self {
        Self { repo, tracker }
    }

    /// Adds issues from an issue tracker to the local database
    ///
    /// This method accepts generic `Issue` from any tracker.
    /// If an issue with the same `key` already exists, it is updated.
    ///
    /// # Arguments
    ///
    /// * `issues` - Slice of issues to add or update
    ///
    /// # Errors
    ///
    /// Returns `WorklogError` if the database operation fails
    pub async fn add_issues(&self, issues: &[Issue]) -> Result<(), WorklogError> {
        self.repo.add_issues(issues).await
    }

    /// Retrieves issues from the database filtered by keys
    ///
    /// # Arguments
    ///
    /// * `keys` - Slice of issue keys to filter by
    ///
    /// # Returns
    ///
    /// Vector of matching issues
    ///
    /// # Errors
    ///
    /// Returns `WorklogError` if the database query fails
    pub async fn get_issues_by_keys(&self, keys: &[String]) -> Result<Vec<Issue>, WorklogError> {
        self.repo.find_by_keys(keys).await
    }

    /// Get a single issue by its key
    ///
    /// # Arguments
    ///
    /// * `key` - The issue key to search for
    ///
    /// # Returns
    ///
    /// The issue if found, None otherwise
    ///
    /// # Errors
    ///
    /// Returns `WorklogError` if the database query fails
    pub async fn get_issue_by_key(&self, key: &str) -> Result<Option<Issue>, WorklogError> {
        self.repo.find_by_key(key).await
    }

    /// Get all issues from local cache
    ///
    /// Returns all issues that have been stored in the local database.
    ///
    /// # Returns
    ///
    /// Vector of all cached issues
    ///
    /// # Errors
    ///
    /// Returns `WorklogError` if the database query fails
    /// Get all cached issues from local storage
    /// Alias for compatibility
    pub async fn get_all_issues(&self) -> Result<Vec<Issue>, WorklogError> {
        self.get_all_cached_issues().await
    }

    /// Get all cached issues from local storage
    pub async fn get_all_cached_issues(&self) -> Result<Vec<Issue>, WorklogError> {
        self.repo.find_all().await
    }

    /// Search for issues by text query in the local cache
    ///
    /// Searches issue keys, summaries, descriptions, and tags.
    ///
    /// # Arguments
    ///
    /// * `query` - Text to search for
    ///
    /// # Returns
    ///
    /// Vector of matching issues
    ///
    /// # Errors
    ///
    /// Returns `WorklogError` if the database query fails
    pub async fn search_issues(&self, query: &str) -> Result<Vec<Issue>, WorklogError> {
        self.repo.search(query).await
    }

    /// Get all unique issue keys that have associated worklog entries
    ///
    /// Useful for finding which issues have been worked on.
    ///
    /// # Returns
    ///
    /// Vector of issue keys
    ///
    /// # Errors
    ///
    /// Returns `WorklogError` if the database query fails
    pub async fn find_keys_with_worklogs(&self) -> Result<Vec<String>, WorklogError> {
        self.repo.find_keys_with_worklogs().await
    }

    /// Delete an issue from the local cache
    ///
    /// # Arguments
    ///
    /// * `key` - The issue key to delete
    ///
    /// # Errors
    ///
    /// Returns `WorklogError` if the database operation fails
    pub async fn delete_issue(&self, key: &str) -> Result<(), WorklogError> {
        self.repo.delete(key).await
    }

    /// Search for issues in a project from the remote issue tracker
    ///
    /// Fetches issues directly from the remote tracker (e.g., Jira) and caches them locally.
    ///
    /// # Arguments
    ///
    /// * `project` - The project key to search in
    /// * `worklog_filter` - Filter for worklog authors:
    ///   - `None` = All issues (no worklog filter)
    ///   - `Some(true)` = Issues with worklogs by any user
    ///   - `Some(false)` = Issues with worklogs by current user only
    ///
    /// # Returns
    ///
    /// Vector of issues found in the project
    ///
    /// # Errors
    ///
    /// Returns `WorklogError` if the remote search fails or local caching fails
    pub async fn search_issues_in_project(
        &self,
        project: &str,
        worklog_filter: Option<bool>,
    ) -> Result<Vec<Issue>, WorklogError> {
        // Fetch from remote tracker
        let issues = self
            .tracker
            .search_issues_in_project(project, worklog_filter)
            .await?;

        // Cache the issues locally
        if !issues.is_empty() {
            self.repo.add_issues(&issues).await?;
        }

        Ok(issues)
    }

    /// Get issues the current user has recently worked on
    ///
    /// Fetches issues from the remote tracker that the current user has logged time to recently,
    /// and caches them locally.
    ///
    /// # Returns
    ///
    /// Vector of recently worked issues
    ///
    /// # Errors
    ///
    /// Returns `WorklogError` if the remote fetch fails or local caching fails
    pub async fn get_recently_worked_issues(&self) -> Result<Vec<Issue>, WorklogError> {
        // Fetch from remote tracker
        let issues = self.tracker.get_recently_worked_issues().await?;

        // Cache the issues locally
        if !issues.is_empty() {
            self.repo.add_issues(&issues).await?;
        }

        Ok(issues)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::traits::{MockIssueRepository, MockIssueTrackerClient};
    use mockall::predicate::*;

    fn create_test_issue(key: &str, summary: &str) -> Issue {
        Issue {
            key: key.to_string(),
            summary: summary.to_string(),
            description: None,
            tags: vec![],
            provider_id: None,
        }
    }

    #[tokio::test]
    async fn test_add_issues_success() {
        let issues = vec![
            create_test_issue("PROJ-1", "First issue"),
            create_test_issue("PROJ-2", "Second issue"),
        ];
        let issues_clone = issues.clone();

        let mut mock_repo = MockIssueRepository::new();
        mock_repo
            .expect_add_issues()
            .withf(move |i| i.len() == 2 && i[0].key == "PROJ-1")
            .times(1)
            .returning(|_| Ok(()));

        let mock_tracker = MockIssueTrackerClient::new();
        let service = IssueService::new(Arc::new(mock_repo), Arc::new(mock_tracker));

        let result = service.add_issues(&issues_clone).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_add_issues_propagates_error() {
        let issues = vec![create_test_issue("PROJ-1", "Test issue")];

        let mut mock_repo = MockIssueRepository::new();
        mock_repo
            .expect_add_issues()
            .times(1)
            .returning(|_| Err(WorklogError::StorageError("Database full".to_string())));

        let mock_tracker = MockIssueTrackerClient::new();
        let service = IssueService::new(Arc::new(mock_repo), Arc::new(mock_tracker));

        let result = service.add_issues(&issues).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Database full"));
    }

    #[tokio::test]
    async fn test_get_issues_by_keys_success() {
        let expected_issues = vec![
            create_test_issue("PROJ-1", "First issue"),
            create_test_issue("PROJ-2", "Second issue"),
        ];
        let issues_clone = expected_issues.clone();

        let mut mock_repo = MockIssueRepository::new();
        mock_repo
            .expect_find_by_keys()
            .with(eq(vec!["PROJ-1".to_string(), "PROJ-2".to_string()]))
            .times(1)
            .returning(move |_| Ok(issues_clone.clone()));

        let mock_tracker = MockIssueTrackerClient::new();
        let service = IssueService::new(Arc::new(mock_repo), Arc::new(mock_tracker));

        let keys = vec!["PROJ-1".to_string(), "PROJ-2".to_string()];
        let result = service.get_issues_by_keys(&keys).await.unwrap();

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].key, "PROJ-1");
        assert_eq!(result[1].key, "PROJ-2");
    }

    #[tokio::test]
    async fn test_get_issues_by_keys_empty() {
        let mut mock_repo = MockIssueRepository::new();
        mock_repo
            .expect_find_by_keys()
            .times(1)
            .returning(|_| Ok(vec![]));

        let mock_tracker = MockIssueTrackerClient::new();
        let service = IssueService::new(Arc::new(mock_repo), Arc::new(mock_tracker));

        let result = service
            .get_issues_by_keys(&["NONEXISTENT".to_string()])
            .await
            .unwrap();

        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn test_get_issue_by_key_found() {
        let expected_issue = create_test_issue("PROJ-1", "Test issue");
        let issue_clone = expected_issue.clone();

        let mut mock_repo = MockIssueRepository::new();
        mock_repo
            .expect_find_by_key()
            .with(eq("PROJ-1"))
            .times(1)
            .returning(move |_| Ok(Some(issue_clone.clone())));

        let mock_tracker = MockIssueTrackerClient::new();
        let service = IssueService::new(Arc::new(mock_repo), Arc::new(mock_tracker));

        let result = service.get_issue_by_key("PROJ-1").await.unwrap();

        assert!(result.is_some());
        let issue = result.unwrap();
        assert_eq!(issue.key, "PROJ-1");
        assert_eq!(issue.summary, "Test issue");
    }

    #[tokio::test]
    async fn test_get_issue_by_key_not_found() {
        let mut mock_repo = MockIssueRepository::new();
        mock_repo
            .expect_find_by_key()
            .with(eq("NONEXISTENT"))
            .times(1)
            .returning(|_| Ok(None));

        let mock_tracker = MockIssueTrackerClient::new();
        let service = IssueService::new(Arc::new(mock_repo), Arc::new(mock_tracker));

        let result = service.get_issue_by_key("NONEXISTENT").await.unwrap();

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_get_all_issues_success() {
        let expected_issues = vec![
            create_test_issue("PROJ-1", "First"),
            create_test_issue("PROJ-2", "Second"),
            create_test_issue("PROJ-3", "Third"),
        ];
        let issues_clone = expected_issues.clone();

        let mut mock_repo = MockIssueRepository::new();
        mock_repo
            .expect_find_all()
            .times(1)
            .returning(move || Ok(issues_clone.clone()));

        let mock_tracker = MockIssueTrackerClient::new();
        let service = IssueService::new(Arc::new(mock_repo), Arc::new(mock_tracker));

        let result = service.get_all_issues().await.unwrap();

        assert_eq!(result.len(), 3);
    }

    #[tokio::test]
    async fn test_get_all_cached_issues_alias() {
        let expected_issues = vec![create_test_issue("PROJ-1", "Test")];
        let issues_clone = expected_issues.clone();

        let mut mock_repo = MockIssueRepository::new();
        mock_repo
            .expect_find_all()
            .times(1)
            .returning(move || Ok(issues_clone.clone()));

        let mock_tracker = MockIssueTrackerClient::new();
        let service = IssueService::new(Arc::new(mock_repo), Arc::new(mock_tracker));

        let result = service.get_all_cached_issues().await.unwrap();

        assert_eq!(result.len(), 1);
    }

    #[tokio::test]
    async fn test_search_issues_success() {
        let expected_issues = vec![
            create_test_issue("PROJ-1", "Bug in authentication"),
            create_test_issue("PROJ-2", "Authentication refactor"),
        ];
        let issues_clone = expected_issues.clone();

        let mut mock_repo = MockIssueRepository::new();
        mock_repo
            .expect_search()
            .with(eq("authentication"))
            .times(1)
            .returning(move |_| Ok(issues_clone.clone()));

        let mock_tracker = MockIssueTrackerClient::new();
        let service = IssueService::new(Arc::new(mock_repo), Arc::new(mock_tracker));

        let result = service.search_issues("authentication").await.unwrap();

        assert_eq!(result.len(), 2);
    }

    #[tokio::test]
    async fn test_search_issues_no_results() {
        let mut mock_repo = MockIssueRepository::new();
        mock_repo
            .expect_search()
            .with(eq("nonexistent"))
            .times(1)
            .returning(|_| Ok(vec![]));

        let mock_tracker = MockIssueTrackerClient::new();
        let service = IssueService::new(Arc::new(mock_repo), Arc::new(mock_tracker));

        let result = service.search_issues("nonexistent").await.unwrap();

        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn test_find_keys_with_worklogs_success() {
        let expected_keys = vec!["PROJ-1".to_string(), "PROJ-2".to_string()];
        let keys_clone = expected_keys.clone();

        let mut mock_repo = MockIssueRepository::new();
        mock_repo
            .expect_find_keys_with_worklogs()
            .times(1)
            .returning(move || Ok(keys_clone.clone()));

        let mock_tracker = MockIssueTrackerClient::new();
        let service = IssueService::new(Arc::new(mock_repo), Arc::new(mock_tracker));

        let result = service.find_keys_with_worklogs().await.unwrap();

        assert_eq!(result.len(), 2);
        assert!(result.contains(&"PROJ-1".to_string()));
        assert!(result.contains(&"PROJ-2".to_string()));
    }

    #[tokio::test]
    async fn test_delete_issue_success() {
        let mut mock_repo = MockIssueRepository::new();
        mock_repo
            .expect_delete()
            .with(eq("PROJ-1"))
            .times(1)
            .returning(|_| Ok(()));

        let mock_tracker = MockIssueTrackerClient::new();
        let service = IssueService::new(Arc::new(mock_repo), Arc::new(mock_tracker));

        let result = service.delete_issue("PROJ-1").await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_delete_issue_propagates_error() {
        let mut mock_repo = MockIssueRepository::new();
        mock_repo
            .expect_delete()
            .times(1)
            .returning(|_| Err(WorklogError::StorageError("Cannot delete".to_string())));

        let mock_tracker = MockIssueTrackerClient::new();
        let service = IssueService::new(Arc::new(mock_repo), Arc::new(mock_tracker));

        let result = service.delete_issue("PROJ-1").await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_search_issues_in_project_fetches_and_caches() {
        let expected_issues = vec![
            create_test_issue("PROJ-1", "Issue 1"),
            create_test_issue("PROJ-2", "Issue 2"),
        ];
        let issues_clone = expected_issues.clone();

        let mut mock_repo = MockIssueRepository::new();
        mock_repo
            .expect_add_issues()
            .withf(move |i| i.len() == 2)
            .times(1)
            .returning(|_| Ok(()));

        let mut mock_tracker = MockIssueTrackerClient::new();
        mock_tracker
            .expect_search_issues_in_project()
            .with(eq("PROJ"), eq(None))
            .times(1)
            .returning(move |_, _| Ok(issues_clone.clone()));

        let service = IssueService::new(Arc::new(mock_repo), Arc::new(mock_tracker));

        let result = service
            .search_issues_in_project("PROJ", None)
            .await
            .unwrap();

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].key, "PROJ-1");
    }

    #[tokio::test]
    async fn test_search_issues_in_project_empty_results_no_cache() {
        let mut mock_repo = MockIssueRepository::new();
        mock_repo.expect_add_issues().times(0); // Should not cache empty results

        let mut mock_tracker = MockIssueTrackerClient::new();
        mock_tracker
            .expect_search_issues_in_project()
            .times(1)
            .returning(|_, _| Ok(vec![]));

        let service = IssueService::new(Arc::new(mock_repo), Arc::new(mock_tracker));

        let result = service
            .search_issues_in_project("PROJ", None)
            .await
            .unwrap();

        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn test_search_issues_in_project_with_worklog_filter() {
        let expected_issues = vec![create_test_issue("PROJ-1", "Issue with worklogs")];
        let issues_clone = expected_issues.clone();

        let mut mock_repo = MockIssueRepository::new();
        mock_repo.expect_add_issues().times(1).returning(|_| Ok(()));

        let mut mock_tracker = MockIssueTrackerClient::new();
        mock_tracker
            .expect_search_issues_in_project()
            .with(eq("PROJ"), eq(Some(true)))
            .times(1)
            .returning(move |_, _| Ok(issues_clone.clone()));

        let service = IssueService::new(Arc::new(mock_repo), Arc::new(mock_tracker));

        let result = service
            .search_issues_in_project("PROJ", Some(true))
            .await
            .unwrap();

        assert_eq!(result.len(), 1);
    }

    #[tokio::test]
    async fn test_search_issues_in_project_tracker_error() {
        let mut mock_repo = MockIssueRepository::new();
        mock_repo.expect_add_issues().times(0);

        let mut mock_tracker = MockIssueTrackerClient::new();
        mock_tracker
            .expect_search_issues_in_project()
            .times(1)
            .returning(|_, _| {
                Err(WorklogError::IssueTrackerError(
                    "Project not found".to_string(),
                ))
            });

        let service = IssueService::new(Arc::new(mock_repo), Arc::new(mock_tracker));

        let result = service.search_issues_in_project("PROJ", None).await;

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Project not found"));
    }

    #[tokio::test]
    async fn test_search_issues_in_project_cache_error() {
        let expected_issues = vec![create_test_issue("PROJ-1", "Issue 1")];
        let issues_clone = expected_issues.clone();

        let mut mock_repo = MockIssueRepository::new();
        mock_repo
            .expect_add_issues()
            .times(1)
            .returning(|_| Err(WorklogError::StorageError("Cache full".to_string())));

        let mut mock_tracker = MockIssueTrackerClient::new();
        mock_tracker
            .expect_search_issues_in_project()
            .times(1)
            .returning(move |_, _| Ok(issues_clone.clone()));

        let service = IssueService::new(Arc::new(mock_repo), Arc::new(mock_tracker));

        let result = service.search_issues_in_project("PROJ", None).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Cache full"));
    }

    #[tokio::test]
    async fn test_get_recently_worked_issues_success() {
        let expected_issues = vec![
            create_test_issue("PROJ-1", "Recent work 1"),
            create_test_issue("PROJ-2", "Recent work 2"),
        ];
        let issues_clone = expected_issues.clone();

        let mut mock_repo = MockIssueRepository::new();
        mock_repo
            .expect_add_issues()
            .withf(move |i| i.len() == 2)
            .times(1)
            .returning(|_| Ok(()));

        let mut mock_tracker = MockIssueTrackerClient::new();
        mock_tracker
            .expect_get_recently_worked_issues()
            .times(1)
            .returning(move || Ok(issues_clone.clone()));

        let service = IssueService::new(Arc::new(mock_repo), Arc::new(mock_tracker));

        let result = service.get_recently_worked_issues().await.unwrap();

        assert_eq!(result.len(), 2);
    }

    #[tokio::test]
    async fn test_get_recently_worked_issues_empty_no_cache() {
        let mut mock_repo = MockIssueRepository::new();
        mock_repo.expect_add_issues().times(0);

        let mut mock_tracker = MockIssueTrackerClient::new();
        mock_tracker
            .expect_get_recently_worked_issues()
            .times(1)
            .returning(|| Ok(vec![]));

        let service = IssueService::new(Arc::new(mock_repo), Arc::new(mock_tracker));

        let result = service.get_recently_worked_issues().await.unwrap();

        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn test_get_recently_worked_issues_tracker_error() {
        let mut mock_repo = MockIssueRepository::new();
        mock_repo.expect_add_issues().times(0);

        let mut mock_tracker = MockIssueTrackerClient::new();
        mock_tracker
            .expect_get_recently_worked_issues()
            .times(1)
            .returning(|| {
                Err(WorklogError::IssueTrackerError(
                    "Network timeout".to_string(),
                ))
            });

        let service = IssueService::new(Arc::new(mock_repo), Arc::new(mock_tracker));

        let result = service.get_recently_worked_issues().await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Network timeout"));
    }
}
