/// A trait for managing work log entries in a storage repository.
///
/// This trait defines methods for adding, removing, querying, and manipulating
/// work log entries in a structured way. Each method provides appropriate
/// documentation about its purpose, input parameters, potential return values,
/// and the errors it might produce.
use crate::error::WorklogError;
use crate::types::{LocalWorklog, Timer};
use chrono::{DateTime, Local};
use jira::models::core::IssueKey;
use jira::models::user::User;
use jira::models::worklog::Worklog;

pub trait TimerRepository: Send + Sync {
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
    fn remove_timer_entry(&self, id: i64) -> Result<(), WorklogError>;

    fn start_timer(&self, timer: &Timer) -> Result<i64, WorklogError>;
    
    fn find_current_timer(&self) -> Result<Option<Timer>, WorklogError>;
}
