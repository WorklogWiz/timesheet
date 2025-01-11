use crate::error::WorklogError;
use crate::types::JiraIssueInfo;
use jira::models::core::IssueKey;
use jira::models::issue::IssueSummary;

pub trait IssueRepository {
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
    fn add_jira_issues(&self, jira_issues: &Vec<IssueSummary>) -> Result<(), WorklogError>;

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
    fn get_issues_filtered_by_keys(
        &self,
        keys: &Vec<IssueKey>,
    ) -> Result<Vec<JiraIssueInfo>, WorklogError>;

    ///
    /// # Errors
    /// Returns an error something goes wrong
    fn find_unique_keys(&self) -> Result<Vec<IssueKey>, WorklogError>;
}
