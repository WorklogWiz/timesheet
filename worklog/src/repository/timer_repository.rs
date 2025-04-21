/// A trait for managing work log entries in a storage repository.
///
/// This trait defines methods for adding, removing, querying, and manipulating
/// work log entries in a structured way. Each method provides appropriate
/// documentation about its purpose, input parameters, potential return values,
/// and the errors it might produce.
use crate::error::WorklogError;
use crate::types::{LocalWorklog, Timer};
use chrono::{DateTime, Local, Utc};
use rusqlite::params;
use jira::models::core::IssueKey;
use jira::models::user::User;
use jira::models::worklog::Worklog;

pub trait TimerRepository: Send + Sync {

    fn start_timer(&self, timer: &Timer) -> Result<i64, WorklogError>;
    
    fn find_active_timer(&self) -> Result<Option<Timer>, WorklogError>;

    fn stop_active_timer(&self) -> Result<Timer, WorklogError>;
    /// Finds all timers for a specific issue
    fn find_by_issue_id(&self, issue_id: &str) -> Result<Vec<Timer>, WorklogError>;
    /// Finds all timers that started after a specific date
     fn find_after_date(&self, date: DateTime<Utc>) -> Result<Vec<Timer>, WorklogError>;
    /// Deletes a timer by its ID
    fn delete(&self, id: i64) -> Result<(), WorklogError>;
    /// Updates an existing timer in the database
     fn update(&self, timer: &Timer) -> Result<(), WorklogError>;
}
