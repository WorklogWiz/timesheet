/// A trait for managing work log entries in a storage repository.
///
/// This trait defines methods for adding, removing, querying, and manipulating
/// work log entries in a structured way. Each method provides appropriate
/// documentation about its purpose, input parameters, potential return values,
/// and the errors it might produce.
use crate::error::WorklogError;
use crate::types::LocalWorklog;
use chrono::{DateTime, Local};
use jira::models::core::IssueKey;
use jira::models::user::User;
use jira::models::worklog::Worklog;

pub trait WorkLogRepository: Send + Sync {
    ///
    /// Removes a worklog entry from the repository.
    ///
    /// # Arguments
    /// * `wl` - A reference to a `Worklog` object to be removed.
    ///
    /// # Returns
    /// * `Ok(())` - If the worklog entry is successfully removed.
    /// * `Err(WorklogError)` - If there is an error while removing the worklog entry.
    ///
    /// # Errors
    /// * This function returns a `WorklogError` if the operation fails.
    fn remove_worklog_entry(&self, wl: &Worklog) -> Result<(), WorklogError>;

    ///
    /// Removes a worklog entry from the repository by its unique identifier.
    ///
    /// # Arguments
    /// * `wl_id` - A reference to a string representing the unique identifier of the worklog entry to be removed.
    ///
    /// # Returns
    /// * `Ok(())` - If the worklog entry is successfully removed.
    /// * `Err(WorklogError)` - If there is an error while removing the worklog entry.
    ///
    /// # Errors
    /// * This function returns a `WorklogError` if the operation fails.
    ///
    fn remove_entry_by_worklog_id(&self, wl_id: &str) -> Result<(), WorklogError>;

    ///
    /// Adds a worklog entry to the repository.
    ///
    /// # Arguments
    /// * `local_worklog` - A reference to a `LocalWorklog` object containing the worklog details to be added.
    ///
    /// # Returns
    /// * `Ok(())` - If the worklog entry is successfully added.
    /// * `Err(WorklogError)` - If there is an error while adding the worklog entry.
    ///
    /// # Errors
    /// * This function returns a `WorklogError` if the operation fails.
    fn add_entry(&self, local_worklog: &LocalWorklog) -> Result<(), WorklogError>;

    ///
    /// Adds multiple worklog entries to the repository.
    ///
    /// # Arguments
    /// * `worklogs` - A slice of `LocalWorklog` objects containing the worklog details to be added.
    ///
    /// # Returns
    /// * `Ok(())` - If all the worklog entries are successfully added.
    /// * `Err(WorklogError)` - If there is an error while adding any of the worklog entries.
    ///
    /// # Errors
    /// * This function returns a `WorklogError` if the operation fails for any entry.
    fn add_worklog_entries(&self, worklogs: &[LocalWorklog]) -> Result<(), WorklogError>;

    ///
    /// Retrieves the total count of worklog entries in the repository.
    ///
    /// # Returns
    /// * `Ok(i64)` - The total count of worklogs as a 64-bit integer.
    /// * `Err(WorklogError)` - If there is an error while retrieving the count.
    ///
    /// # Errors
    /// * Returns a `WorklogError` if the operation fails.
    fn get_count(&self) -> Result<i64, WorklogError>;

    ///
    /// Deletes all worklog entries from the repository.
    ///
    /// # Returns
    /// * `Ok(())` - If all worklog entries are successfully deleted.
    /// * `Err(WorklogError)` - If there is an error during the deletion process.
    ///
    /// # Errors
    /// This function returns a `WorklogError` if the operation fails for any reason.
    fn purge_entire_local_worklog(&self) -> Result<(), WorklogError>;

    ///
    /// Finds a worklog entry by its identifier.
    ///
    /// # Arguments
    /// * `worklog_id` - A reference to a string representing the unique identifier of the worklog entry to be found.
    ///
    /// # Returns
    /// * `Ok(LocalWorklog)` - A `LocalWorklog` object if the entry with the specified ID is found.
    /// * `Err(WorklogError)` - If there is an error during the retrieval process or the entry is not found.
    ///
    /// # Errors
    /// * Returns a `WorklogError` if the operation fails.
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
