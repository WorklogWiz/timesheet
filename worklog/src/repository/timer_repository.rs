/// A trait for managing work log entries in a storage repository.
///
/// This trait defines methods for adding, removing, querying, and manipulating
/// work log entries in a structured way. Each method provides appropriate
/// documentation about its purpose, input parameters, potential return values,
/// and the errors it might produce.
use crate::error::WorklogError;
use crate::types::Timer;
use chrono::{DateTime, Local, Utc};

/// Repository trait for managing work time tracking timers.
///
/// This trait provides thread-safe operations for managing timer entries,
/// including starting, stopping, querying, and updating timers.
/// Implementations must be both Send and Sync to ensure thread-safety.
pub trait TimerRepository: Send + Sync {
    /// Starts a new timer and stores it in the repository.
    ///
    /// # Arguments
    /// * `timer` - The timer instance to start and store
    ///
    /// # Returns
    /// * The ID of the newly created timer entry
    fn start_timer(&self, timer: &Timer) -> Result<i64, WorklogError>;

    /// Retrieves the currently active timer, if one exists.
    ///
    /// # Returns
    /// * `Some(Timer)` if an active timer exists
    /// * `None` if no timer is currently active
    fn find_active_timer(&self) -> Result<Option<Timer>, WorklogError>;

    /// Stops the currently active timer by setting its end time.
    ///
    /// # Arguments
    /// * `stop_time` - The timestamp when the timer was stopped
    ///
    /// # Returns
    /// * The updated timer entry after being stopped
    fn stop_active_timer(&self, stop_time: DateTime<Local>) -> Result<Timer, WorklogError>;

    /// Finds all timers for a specific issue
    fn find_by_issue_key(&self, issue_key: &str) -> Result<Vec<Timer>, WorklogError>;
    /// Finds all timers that started after a specific date
    fn find_after_date(&self, date: DateTime<Utc>) -> Result<Vec<Timer>, WorklogError>;
    /// Deletes a timer by its ID
    fn delete(&self, id: i64) -> Result<(), WorklogError>;
    /// Updates an existing timer in the database
    fn update(&self, timer: &Timer) -> Result<(), WorklogError>;
}
