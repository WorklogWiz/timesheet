use crate::error::WorklogError;
use crate::repository::timer_repository::TimerRepository;
use crate::service::issue::IssueService;
use crate::service::worklog::WorkLogService;
use crate::types::{LocalWorklog, Timer};
use chrono::{DateTime, Duration, Local, Utc};
use jira::models::core::IssueKey;
use jira::Jira;
use num_traits::ToPrimitive;
use std::sync::Arc;
use log::debug;

/// Service for managing timer operations and synchronization with Jira
pub struct TimerService {
    timer_repository: Arc<dyn TimerRepository>,
    issue_service: Arc<IssueService>,
    worklog_service: Arc<WorkLogService>,
    jira_client: Jira,
}

impl TimerService {
    /// Creates a new TimerService instance
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

    /// Starts a new timer for the specified issue
    ///
    /// Validates that the issue exists before starting the timer
    pub fn start_timer(
        &self,
        issue_key: &str,
        comment: Option<String>,
    ) -> Result<Timer, WorklogError> {
        // Validate that the issue exists in our database
        let issue_key = IssueKey::new(issue_key);
        let issues = self
            .issue_service
            .get_issues_filtered_by_keys(&[issue_key.clone()])?;

        if issues.is_empty() {
            return Err(WorklogError::IssueNotFound(issue_key.value().to_string()));
        }

        // Check if there's already an active timer
        if self.timer_repository.find_active_timer()?.is_some() {
            return Err(WorklogError::ActiveTimerExists);
        }

        // Create a new timer with the current time
        let now = Utc::now();
        let timer = Timer {
            id: None,
            issue_key: issue_key.to_string(),
            created_at: now.with_timezone(&Local),
            started_at: now.with_timezone(&Local),
            stopped_at: None,
            synced: false,
            comment,
        };

        // Start the timer and get its ID
        let timer_id = self.timer_repository.start_timer(&timer)?;
        debug!("Started timer with ID: {}", timer_id);
        
        // Return the timer with its ID
        Ok(Timer {
            id: Some(timer_id),
            ..timer
        })
    }

    /// Stops the currently active timer
    pub fn stop_active_timer(&self, stop_time: Option<DateTime<Local>>) -> Result<Timer, WorklogError> {
        self.timer_repository.stop_active_timer(stop_time)
    }

    /// Gets the currently active timer, if any
    pub fn get_active_timer(&self) -> Result<Option<Timer>, WorklogError> {
        self.timer_repository.find_active_timer()
    }

    /// Synchronizes completed and unsynced timers with Jira as worklogs
    pub async fn sync_timers_to_jira(&self) -> Result<Vec<Timer>, WorklogError> {
        debug!("Syncing timers to Jira");
        // Find timers that have been stopped but not synced
        let timers = self.find_unsynced_completed_timers()?;
        debug!("Found {} unsynced timers", timers.len());
        
        let mut synced_timers = Vec::new();

        for mut timer in timers {
            debug!("Syncing timer: {:?}", timer);
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
                let work_log = self.jira_client.insert_worklog(
                    &timer.issue_key,
                    timer.started_at.with_timezone(&Local),
                    duration_seconds.to_i32().unwrap(),
                    comment,
                ).await?;
                
                debug!("Worklog created in Jira: {:?}", work_log);
                
                // Write to local worklog database table too
                self.worklog_service.add_entry(&LocalWorklog::from_worklog(&work_log, &IssueKey::from(timer.issue_key.as_str())))?;
                
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
            .find_after_date(Utc::now() - chrono::Duration::days(30))?;

        let unsynced_completed = all_timers
            .into_iter()
            .filter(|timer| !timer.synced && timer.stopped_at.is_some())
            .collect();

        Ok(unsynced_completed)
    }

    /// Gets the total time spent on an issue
    pub fn get_total_time_for_issue(&self, issue_id: &str) -> Result<Duration, WorklogError> {
        let timers = self.timer_repository.find_by_issue_key(issue_id)?;

        let mut total = Duration::seconds(0);
        for timer in timers {
            if let Some(stopped_at) = timer.stopped_at {
                total = total + (stopped_at - timer.started_at);
            } else {
                // For active timers, calculate duration up to now
                total = total + (Utc::now().with_timezone(&Local) - timer.started_at);
            }
        }

        Ok(total)
    }

    /// Discards the currently active timer
    pub fn discard_active_timer(&self) -> Result<(), WorklogError> {
        let active_timer = self.get_active_timer()?;
        if let Some(timer) = active_timer {
            if let Some(id) = timer.id {
                self.timer_repository.delete(id)?;
                Ok(())
            } else {
                Err(WorklogError::InvalidTimerData(
                    "Timer has no ID".to_string(),
                ))
            }
        } else {
            Err(WorklogError::NoActiveTimer)
        }
    }

    /// Updates a timer's comment
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
            .find_after_date(Utc::now() - chrono::Duration::days(365))?;

        timers
            .into_iter()
            .find(|t| t.id == Some(timer_id))
            .ok_or_else(|| WorklogError::TimerNotFound(timer_id))
    }

    /// Gets all recent timers for a specific issue
    pub fn get_recent_timers_for_issue(
        &self,
        issue_id: &str,
        days: i64,
    ) -> Result<Vec<Timer>, WorklogError> {
        let since = Utc::now() - chrono::Duration::days(days);
        let all_timers = self.timer_repository.find_after_date(since)?;

        Ok(all_timers
            .into_iter()
            .filter(|t| t.issue_key == issue_id)
            .collect())
    }
}
