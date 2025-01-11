use chrono::{DateTime, Local};
use jira::models::core::IssueKey;
use jira::models::user::User;
use jira::models::worklog::Worklog;
use crate::error::WorklogError;
use crate::types::LocalWorklog;

pub trait WorkLogRepository {
    ///
    /// # Errors
    /// Returns an error something goes wrong
    fn remove_worklog_entry(&self, wl: &Worklog) -> Result<(), WorklogError>;
    ///
    /// # Errors
    /// Returns an error something goes wrong
    fn remove_entry_by_worklog_id(&self, wl_id: &str) -> Result<(), WorklogError>;
    /// Adds a new work log entry into the local DBMS
    ///
    /// # Errors
    /// Returns an error something goes wrong
    fn add_entry(&self, local_worklog: &LocalWorklog) -> Result<(), WorklogError>;
    ///
    /// # Errors
    /// Returns an error something goes wrong
    fn add_worklog_entries(&self, worklogs: &[LocalWorklog]) -> Result<(), WorklogError>;
    ///
    /// # Errors
    /// Returns an error something goes wrong
    fn get_count(&self) -> Result<i64, WorklogError>;
    ///
    /// # Errors
    /// Returns an error something goes wrong
    fn purge_entire_local_worklog(&self) -> Result<(), WorklogError>;
    ///
    /// # Errors
    /// Returns an error something goes wrong
    ///
    /// # Panics
    /// If the worklog id could not be parsed into an integer
    ///
    fn find_worklog_by_id(&self, worklog_id: &str) -> Result<LocalWorklog, WorklogError>;
    /// Finds worklog entries that were started after the given `start_datetime` and optionally,
    /// filters them by a list of `keys_filter` (issue keys) and `users_filter` (authors).
    ///
    /// # Arguments
    /// * `start_datetime` - A `DateTime` representing the lower bound for the `started` field.
    /// * `keys_filter` - A slice of `IssueKey` objects to filter the worklogs by their associated issue keys.
    ///   If empty, no filtering on issue keys is done.
    /// * `users_filter` - A slice of `User` objects to filter the worklogs by their associated authors.
    ///   If empty, no filtering on authors is done.
    ///
    /// # Returns
    /// A `Result` containing a `Vec` of `LocalWorklog` objects that match the criteria, or a `WorklogError`
    /// if something goes wrong during query execution.
    ///
    /// # Errors
    /// * Returns a `WorklogError` if the database query fails for any reason.
    ///
    /// # Examples
    /// ```rust,ignore
    /// use chrono::prelude::*;
    /// use crate::storage::dbms::{DbConnector, IssueKey, User};
    ///
    /// let db = DbConnector::new("test.db")?;
    /// let start_time = Local::now() - chrono::Duration::days(7);
    /// let issue_keys = vec![IssueKey::from("TEST-123")];
    /// let users = vec![User::new("John Doe".to_string())];
    ///
    /// let result = db.find_worklogs_after(start_time, &issue_keys, &users);
    ///
    /// match result {
    ///     Ok(worklogs) => println!("Retrieved {} worklogs.", worklogs.len()),
    ///     Err(e) => eprintln!("Error: {}", e),
    /// }
    /// ```
    fn find_worklogs_after(
        &self,
        start_datetime: DateTime<Local>,
        keys_filter: &[IssueKey],
        users_filter: &[User],
    ) -> Result<Vec<LocalWorklog>, WorklogError>;
}
