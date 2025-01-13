//! This module contains the `IssueService` struct and its associated methods for managing Jira issues.
//!
//! The `IssueService` provides various functionalities for interacting with a local database of Jira issues.
//! It abstracts common operations such as adding, retrieving, and fetching unique keys for Jira issues,
//! delegating most of the actual database operations to an `IssueRepository`.
//!
//! # Structs
//!
//! - `IssueService`: A service layer that provides methods for adding and retrieving Jira issues.
//!
//! # Errors
//!
//! All methods in this module use the `WorklogError` to report errors encountered during database operations.
//!
//! # Dependencies
//!
//! This module depends on the following external crates:
//! - `jira`: For handling Jira-related structures such as `IssueKey` and `IssueSummary`.
//! - `std::sync::Arc`: For handling thread-safe references to the `IssueRepository`.
//!
//! # Examples
//!
//! Please see individual method documentation for usage examples.
use crate::error::WorklogError;
use crate::repository::issue_repository::IssueRepository;
use crate::types::JiraIssueInfo;
use jira::models::core::IssueKey;
use jira::models::issue::IssueSummary;
use std::sync::Arc;

#[allow(clippy::module_name_repetitions)]
pub struct IssueService {
    repo: Arc<dyn IssueRepository>,
}

#[allow(clippy::module_name_repetitions)]
impl IssueService {
    pub fn new(repo: Arc<dyn IssueRepository>) -> Self {
        Self { repo }
    }
    ///
    /// Adds multiple Jira issues to the local database.
    ///
    /// This function inserts Jira issues into the `issue` table of the local database.
    /// If an issue with the same `key` already exists, its `summary` is updated.
    ///
    /// # Arguments
    ///
    /// * `jira_issues` - A vector of `IssueSummary` objects to be added to the database.
    ///
    /// # Errors
    ///
    /// Returns a `WorklogError` if any SQL operation fails during the insertion or update.
    ///
    /// # Panics
    ///
    /// This method panics if any SQL statement execution fails due to unexpected conditions.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let issues = vec![
    ///     IssueSummary { key: IssueKey::new("ISSUE-1"), fields: Fields { summary: "Issue 1".to_string() } },
    ///     IssueSummary { key: IssueKey::new("ISSUE-2"), fields: Fields { summary: "Issue 2".to_string() } },
    /// ];
    ///
    /// worklog_storage.add_jira_issues(&issues)?;
    /// ```
    pub fn add_jira_issues(&self, jira_issues: &[IssueSummary]) -> Result<(), WorklogError> {
        self.repo.add_jira_issues(jira_issues)
    }

    ///
    /// Retrieves a list of issues from the database filtered by the provided issue keys.
    ///
    /// This function queries the local database for issues whose keys match those
    /// provided in the `keys` parameter. It dynamically constructs the SQL query
    /// to handle a variable number of keys using placeholders. If no keys are provided,
    /// it will return an empty vector.
    ///
    /// # Arguments
    ///
    /// * `keys` - A vector of issue keys of type `IssueKey` to filter the issues.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing a vector of `JiraIssueInfo` objects representing the
    /// matching issues. If an error occurs while querying the database, a `WorklogError` is returned.
    ///
    /// # Errors
    ///
    /// This function may return a `WorklogError` if an error occurs while preparing or
    /// executing the SQL statement, or while processing the result rows.
    ///
    /// # Examples
    ///
    /// ```rust,ignore  
    /// let issue_keys = vec![IssueKey::new("ISSUE-1"), IssueKey::new("ISSUE-2")];
    /// let issues = worklog_storage.get_issues_filtered_by_keys(&issue_keys)?;
    ///
    /// for issue in issues {
    ///     println!("Issue Key: {}, Summary: {}", issue.key.value(), issue.summary);
    /// }
    /// ```
    ///
    pub fn get_issues_filtered_by_keys(
        &self,
        keys: &[IssueKey],
    ) -> Result<Vec<JiraIssueInfo>, WorklogError> {
        self.repo.get_issues_filtered_by_keys(keys)
    }

    ///
    /// Retrieves all unique issue keys from the local database.
    ///
    /// This function queries the database for unique issue keys stored in the `issue`
    /// table. It is useful for obtaining a distinct list of all issue keys.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing a vector of `IssueKey` objects. If an error occurs
    /// during the query execution, a `WorklogError` is returned.
    ///
    /// # Errors
    ///
    /// This function may return a `WorklogError` when:
    /// - There is an error in preparing or executing the SQL query.
    /// - There is an issue processing the result rows fetched from the database.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let unique_keys = worklog_storage.find_unique_keys()?;
    ///
    /// for key in unique_keys {
    ///     println!("Unique Issue Key: {}", key.value());
    /// }
    /// ```
    pub fn find_unique_keys(&self) -> Result<Vec<IssueKey>, WorklogError> {
        self.repo.find_unique_keys()
    }
}
