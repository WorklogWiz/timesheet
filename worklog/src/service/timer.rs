//! Timer service module provides functionality for managing work time tracking.
//!
//! This module contains the `TimerService`, which handles:
//! - Starting and stopping work timers
//! - Synchronizing completed timers with Jira as worklogs
//! - Managing timer states and persistence
//! - Calculating time spent on issues
//! - Timer comment management
//!
//! # Basic Usage Example
//! ```no_run
//! use chrono::Local;
//! use worklog::ApplicationRuntimeBuilder;
//!
//! # async fn run() -> Result<(), Box<dyn std::error::Error>> {
//! // Initialize application runtime
//! let runtime = ApplicationRuntimeBuilder::new()
//!     .build()?;
//!
//! // Start a timer for an issue
//! let timer = runtime.timer_service().start_timer(
//!     "PROJECT-123", Local::now(),
//!     Some("Working on feature".to_string())
//! ).await?;
//!
//! // Do some work...
//!
//! // Stop the timer when done
//! let stopped_timer = runtime.timer_service().stop_active_timer(Local::now(),None)?;
//!
//! // Sync completed timers with Jira
//! runtime.timer_service().sync_timers_to_jira().await?;
//!
//! // Check total time spent on an issue
//! let total_time = runtime.timer_service()
//!     .get_total_time_for_issue("PROJECT-123")?;
//!
//! // Update timer comment
//! if let Some(timer_id) = stopped_timer.id {
//!     runtime.timer_service().update_timer_comment(
//!         timer_id,
//!         Some("Updated work description".to_string())
//!     )?;
//! }
//! # Ok(())
//! # }
//! ```

use crate::error::WorklogError;
use crate::repository::timer_repository::TimerRepository;
use crate::service::issue::IssueService;
use crate::service::worklog::WorkLogService;
use crate::types::{LocalWorklog, Timer};
use chrono::{DateTime, Duration, Local, Utc};
use jira::models::core::IssueKey;
use jira::JiraError::WorklogDurationTooShort;
use jira::{Jira, JiraError};
use log::debug;
use num_traits::ToPrimitive;
use std::sync::Arc;

/// Service for managing timer operations and synchronization with Jira worklogs
///
/// The `TimerService` provides functionality for:
/// - Starting and stopping work timers
/// - Tracking time spent on Jira issues
/// - Syncing completed timers to Jira as worklogs
/// - Managing timer comments and metadata
///
/// # Fields
/// * `timer_repository` - Repository for persisting and retrieving timer data
/// * `issue_service` - Service for managing Jira issue data
/// * `worklog_service` - Service for managing worklog entries
/// * `jira_client` - Client for interacting with Jira API
///
/// # Example
/// ```no_run
/// use chrono::Local;
/// use worklog::TimerService;
///
/// # async fn example(timer_service: TimerService) -> Result<(), Box<dyn std::error::Error>> {
/// // Start a timer for an issue
/// let timer = timer_service.start_timer("PROJECT-123", Local::now(),Some("Implementing feature".into())).await?;
///
/// // Work on the issue...
///
/// // Stop the timer
/// let completed_timer = timer_service.stop_active_timer(Local::now(),None)?;
///
/// // Sync completed timer to Jira
/// timer_service.sync_timers_to_jira().await?;
/// # Ok(())
/// # }
/// ```
pub struct TimerService {
    timer_repository: Arc<dyn TimerRepository>,
    issue_service: Arc<IssueService>,
    worklog_service: Arc<WorkLogService>,
    jira_client: Jira,
}

impl TimerService {
    /// Creates a new `TimerService` instance
    pub fn new(
        timer_repository: Arc<dyn TimerRepository>,
        issue_service: Arc<IssueService>,
        worklog_service: Arc<WorkLogService>,
        jira_client: Jira,
    ) -> Self {
        Self {
            timer_repository,
            issue_service,
            worklog_service,
            jira_client,
        }
    }

    /// Starts a new timer for the specified issue. Creates an entry in
    /// the local database. No requests are sent to Jira
    ///
    /// Validates that the issue exists before starting the timer
    ///
    /// # Errors
    /// Return a `WorklogError` if:
    /// - The issue does not exist in the database
    /// - There is already an active timer running
    /// - There's an error accessing the timer repository
    /// - Database operations fail
    /// - an Issue key format is invalid
    pub async fn start_timer(
        &self,
        issue_key: &str,
        started_at: DateTime<Local>,
        comment: Option<String>,
    ) -> Result<Timer, WorklogError> {
        let issue_key = IssueKey::new(issue_key);

        debug!("Starting timer for issue: {issue_key}");

        // Check if the issue exists in Jira, if not, return an error
        match self.jira_client.get_issue_summary(&issue_key).await {
            Ok(issue_summary) => {
                // Issue exists in Jira, check if it exists in the local database
                debug!("Issue for key {issue_key} found, now checking local database");
                if self
                    .issue_service
                    .get_issues_filtered_by_keys(&[issue_key.clone()])?
                    .is_empty()
                {
                    self.issue_service.add_jira_issues(&[issue_summary])?;
                }
            }
            Err(JiraError::NotFound(k)) => return Err(WorklogError::IssueNotFound(k)),
            Err(e) => {
                return Err(WorklogError::JiraError(e.to_string()));
            }
        }

        // Check if there's already an active timer
        if self.timer_repository.find_active_timer()?.is_some() {
            return Err(WorklogError::ActiveTimerExists);
        }

        // Create a new timer with the current time as the creation
        let timer = Timer {
            id: None,
            issue_key: issue_key.to_string(),
            created_at: Local::now(),
            started_at,
            stopped_at: None,
            synced: false,
            comment,
        };

        // Start the timer and get its ID
        let timer_id = self.timer_repository.start_timer(&timer)?;
        debug!(
            "Started timer with ID: {timer_id} for issue {issue_key}, starting time: {started_at}"
        );

        // Return the timer with its ID
        Ok(Timer {
            id: Some(timer_id),
            ..timer
        })
    }

    /// Stops the currently active timer if one exists. The corresponding
    /// entry in the worklog database is also updated. No requests are sent
    /// to Jira. See also [`TimerService::sync_timers_to_jira`]
    ///
    /// # Arguments
    /// * `stop_time` - Optional custom stop time. If None, current time is used
    ///
    /// # Returns
    /// Returns the stopped timer on success
    ///
    /// # Errors
    /// Returns a `WorklogError` if:
    /// - No active timer exists
    /// - There's an error accessing the timer repository
    /// - Database operations fail
    ///
    /// # Panics
    /// This method will panic if the timer duration in seconds cannot be converted to i32
    pub fn stop_active_timer(
        &self,
        stop_time: DateTime<Local>,
        comment: Option<String>,
    ) -> Result<Timer, WorklogError> {
        // Retrieves the current timer
        let timer = self
            .get_active_timer()?
            .ok_or(WorklogError::NoActiveTimer)?;

        // Calculates the duration of the timer using either a supplied
        // stop time or the current time
        let duration = stop_time - timer.started_at;

        if duration < Duration::seconds(60) {
            return Err(WorklogError::TimerDurationTooSmall(
                duration.num_seconds().to_i32().unwrap(),
            ));
        }

        self.timer_repository.stop_active_timer(stop_time, comment)
    }

    /// Gets the currently active timer, if any
    ///
    /// Returns the currently active timer if one exists, or None if no timer is active.
    ///
    /// # Returns
    /// - `Ok(Some(Timer))` if an active timer exists
    /// - `Ok(None)` if no timer is currently active
    ///
    /// # Errors
    /// Returns a `WorklogError` if:
    /// - There's an error accessing the timer repository
    /// - Database operations fail
    pub fn get_active_timer(&self) -> Result<Option<Timer>, WorklogError> {
        self.timer_repository.find_active_timer()
    }

    /// Synchronizes completed and unsynced timers with Jira as worklogs
    ///
    /// Finds all completed timers that haven't been synced to Jira yet and creates
    /// corresponding worklogs in Jira. Also updates local worklog database and marks
    /// timers as synced upon successful synchronization.
    ///
    /// # Returns
    /// Returns a vector of successfully synced timers
    ///
    /// # Errors
    /// Returns a `WorklogError` if:
    /// - There's an error accessing the timer repository
    /// - There's an error connecting to Jira
    /// - Creating worklogs in Jira fails
    /// - Adding entries to local worklog database fails
    /// - Updating timer sync status fails
    /// - Database operations fail
    ///
    /// # Panics
    /// This method will panic if:
    /// - The duration in seconds cannot be converted to i32
    /// - The timer data is corrupted or invalid
    pub async fn sync_timers_to_jira(&self) -> Result<Vec<Timer>, WorklogError> {
        debug!("Syncing timers to Jira");
        // Find timers that have been stopped but not synced
        let timers = self.find_unsynced_completed_timers()?;
        debug!("Found {} unsynced timers", timers.len());

        let mut synced_timers = Vec::new();

        for mut timer in timers {
            debug!("Syncing timer: {timer:?}");
            if let Some(stopped_at) = timer.stopped_at {
                // Calculate duration in seconds
                let duration_seconds = (stopped_at - timer.started_at).num_seconds();

                // Skip timers with zero or negative duration (shouldn't happen but let's be safe)
                if duration_seconds <= 0 {
                    continue;
                }

                // Create a worklog to send to Jira
                let comment = timer.comment.as_deref().unwrap_or("");

                // Submit worklog to Jira via the jira service
                let work_log = match self
                    .jira_client
                    .insert_worklog(
                        &timer.issue_key,
                        timer.started_at.with_timezone(&Local),
                        duration_seconds.to_i32().unwrap(),
                        comment,
                    )
                    .await
                {
                    Ok(wl) => wl,
                    Err(e) => {
                        if let WorklogDurationTooShort(duration) = e {
                            // Log it and continue
                            eprintln!(
                                    "Worklog duration too short, skipping: timer_id: {} issue:{} duration:{} seconds",
                                    &timer.id.unwrap(),
                                    &timer.issue_key,
                                    duration
                                );
                            continue;
                        }
                        eprintln!(
                                    "Error creating worklog: timer_id: {} issue:{} start:{} duration:{} comment:{} {e} ",
                                    &timer.id.unwrap(),
                                    &timer.issue_key,
                                    &timer.started_at.with_timezone(&Local),
                                    duration_seconds.to_i32().unwrap(),
                                    comment
                                );
                        return Err(WorklogError::JiraError(e.to_string()));
                    }
                };

                debug!("Worklog created in Jira: {work_log:?}");

                // Write to local worklog database table too
                self.worklog_service
                    .add_entry(&LocalWorklog::from_worklog(
                        &work_log,
                        &IssueKey::from(timer.issue_key.as_str()),
                    ))
                    .await?;

                // Mark timer as synced
                timer.synced = true;
                self.timer_repository.update(&timer)?;

                synced_timers.push(timer);
            }
        }

        Ok(synced_timers)
    }

    /// Finds all timers that have been completed but not synced with Jira
    fn find_unsynced_completed_timers(&self) -> Result<Vec<Timer>, WorklogError> {
        // Implementation would depend on your repository capabilities
        // This is a placeholder - you would need to implement this in the repository

        let all_timers = self
            .timer_repository
            .find_after_date(Utc::now() - Duration::days(30))?;

        let unsynced_completed = all_timers
            .into_iter()
            .filter(|timer| !timer.synced && timer.stopped_at.is_some())
            .collect();

        Ok(unsynced_completed)
    }

    /// Gets the total time spent on an issue by summing up all timer durations
    ///
    /// # Arguments
    /// * `issue_id` - The ID or key of the Jira issue to calculate total time for
    ///
    /// # Returns
    /// Returns a `Result` containing the total `Duration` spent on the issue
    ///
    /// # Errors
    /// Returns a `WorklogError` if:
    /// - There's an error accessing the timer repository
    /// - Database operations fail
    /// - Issue ID format is invalid
    pub fn get_total_time_for_issue(&self, issue_id: &str) -> Result<Duration, WorklogError> {
        let timers = self.timer_repository.find_by_issue_key(issue_id)?;

        let mut total = Duration::seconds(0);
        for timer in timers {
            if let Some(stopped_at) = timer.stopped_at {
                total += stopped_at - timer.started_at;
            } else {
                // For active timers, calculate duration up to now
                total += Utc::now().with_timezone(&Local) - timer.started_at;
            }
        }

        Ok(total)
    }

    /// Discards the currently active timer
    ///
    /// # Errors
    /// Returns a `WorklogError` if:
    /// - There is no active timer
    /// - The active timer has no ID
    /// - There's an error accessing the timer repository
    /// - Database operations fail
    pub fn discard_active_timer(&self) -> Result<Timer, WorklogError> {
        let active_timer = self.get_active_timer()?;
        if let Some(timer) = active_timer {
            if let Some(id) = timer.id {
                self.timer_repository.delete(id)?;
                Ok(timer)
            } else {
                Err(WorklogError::InvalidTimerData(
                    "Internal error: Timer has no ID".to_string(),
                ))
            }
        } else {
            Err(WorklogError::NoActiveTimer)
        }
    }

    /// Updates a timer's comment
    ///
    /// # Errors
    /// Returns a `WorklogError` if:
    /// - The timer with given ID is not found
    /// - There's an error accessing the timer repository
    /// - Database operations fail
    pub fn update_timer_comment(
        &self,
        timer_id: i64,
        comment: Option<String>,
    ) -> Result<Timer, WorklogError> {
        // Find the timer
        let mut timer = self.find_timer_by_id(timer_id)?;

        // Update the comment
        timer.comment = comment;

        // Save the changes
        self.timer_repository.update(&timer)?;

        Ok(timer)
    }

    /// Finds a timer by its ID
    fn find_timer_by_id(&self, timer_id: i64) -> Result<Timer, WorklogError> {
        // This would need to be implemented in the repository
        // Placeholder implementation
        let timers = self
            .timer_repository
            .find_after_date(Utc::now() - Duration::days(365))?;

        timers
            .into_iter()
            .find(|t| t.id == Some(timer_id))
            .ok_or(WorklogError::TimerNotFound(timer_id))
    }

    /// Gets all recent timers for a specific issue
    ///
    /// # Errors
    /// Returns a `WorklogError` if:
    /// - There's an error accessing the timer repository
    /// - Database operations fail
    pub fn get_recent_timers_for_issue(
        &self,
        issue_id: &str,
        days: i64,
    ) -> Result<Vec<Timer>, WorklogError> {
        let since = Utc::now() - Duration::days(days);
        let all_timers = self.timer_repository.find_after_date(since)?;

        Ok(all_timers
            .into_iter()
            .filter(|t| t.issue_key == issue_id)
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Timer;
    use chrono::Local;

    #[test]
    fn test_timer_struct_creation() {
        let timer = Timer {
            id: Some(1),
            issue_key: "TEST-123".to_string(),
            created_at: Local::now(),
            started_at: Local::now(),
            stopped_at: None,
            synced: false,
            comment: Some("Test comment".to_string()),
        };

        assert_eq!(timer.id, Some(1));
        assert_eq!(timer.issue_key, "TEST-123");
        assert!(!timer.synced);
        assert_eq!(timer.comment, Some("Test comment".to_string()));
        assert!(timer.stopped_at.is_none());
    }

    #[test]
    fn test_timer_duration_calculation() {
        let start_time = Local::now();
        let stop_time = start_time + Duration::minutes(30);

        let timer = Timer {
            id: Some(1),
            issue_key: "TEST-123".to_string(),
            created_at: start_time,
            started_at: start_time,
            stopped_at: Some(stop_time),
            synced: false,
            comment: None,
        };

        if let Some(duration) = timer.duration() {
            assert_eq!(duration.num_minutes(), 30);
        } else {
            panic!("Timer should have a duration");
        }
    }

    #[test]
    fn test_timer_active_duration() {
        let start_time = Local::now() - Duration::minutes(15);

        let timer = Timer {
            id: Some(1),
            issue_key: "TEST-123".to_string(),
            created_at: start_time,
            started_at: start_time,
            stopped_at: None,
            synced: false,
            comment: None,
        };

        // Active timer should have no duration until stopped
        assert!(timer.duration().is_none());
    }

    #[test]
    fn test_duration_calculation() {
        let start_time = Local::now();
        let end_time = start_time + Duration::hours(2);
        let duration = end_time - start_time;

        assert_eq!(duration.num_hours(), 2);
        assert_eq!(duration.num_seconds(), 7200);
    }

    #[test]
    fn test_timer_with_different_issue_keys() {
        let test_cases = vec!["PROJ-123", "ABC-1", "LONGPROJECT-9999", "X-1"];

        for issue_key in test_cases {
            let timer = Timer {
                id: Some(1),
                issue_key: issue_key.to_string(),
                created_at: Local::now(),
                started_at: Local::now(),
                stopped_at: None,
                synced: false,
                comment: None,
            };
            assert_eq!(timer.issue_key, issue_key);
        }
    }

    #[test]
    fn test_timer_sync_states() {
        let mut timer = Timer {
            id: Some(1),
            issue_key: "TEST-123".to_string(),
            created_at: Local::now(),
            started_at: Local::now(),
            stopped_at: None,
            synced: false,
            comment: None,
        };

        // Initially not synced
        assert!(!timer.synced);

        // Mark as synced
        timer.synced = true;
        assert!(timer.synced);
    }

    #[test]
    fn test_timer_comment_handling() {
        let timer_with_comment = Timer {
            id: Some(1),
            issue_key: "TEST-123".to_string(),
            created_at: Local::now(),
            started_at: Local::now(),
            stopped_at: None,
            synced: false,
            comment: Some("Working on feature".to_string()),
        };

        let timer_without_comment = Timer {
            id: Some(2),
            issue_key: "TEST-456".to_string(),
            created_at: Local::now(),
            started_at: Local::now(),
            stopped_at: None,
            synced: false,
            comment: None,
        };

        assert_eq!(
            timer_with_comment.comment,
            Some("Working on feature".to_string())
        );
        assert!(timer_without_comment.comment.is_none());
    }

    #[test]
    fn test_timer_time_calculations() {
        let base_time = Local::now();
        let start_time = base_time;
        let stop_time = base_time + Duration::hours(1) + Duration::minutes(30);

        let timer = Timer {
            id: Some(1),
            issue_key: "TEST-123".to_string(),
            created_at: base_time,
            started_at: start_time,
            stopped_at: Some(stop_time),
            synced: false,
            comment: None,
        };

        if let Some(duration) = timer.duration() {
            assert_eq!(duration.num_hours(), 1);
            assert_eq!(duration.num_minutes(), 90); // 1 hour 30 minutes = 90 minutes
        } else {
            panic!("Timer should have a duration");
        }
    }
}
