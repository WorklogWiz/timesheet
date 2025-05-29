use chrono::Utc;
use chrono::{DateTime, Local};
use jira::models::core::Author;
use jira::models::core::IssueKey;
use jira::models::worklog::Worklog;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, PartialOrd, Ord, Clone)]
#[allow(non_snake_case)]
#[allow(clippy::module_name_repetitions)]
pub struct LocalWorklog {
    pub issue_key: IssueKey,
    pub id: String, // Numeric, really
    pub author: String,
    pub created: DateTime<Local>,
    pub updated: DateTime<Local>,
    pub started: DateTime<Local>,
    pub timeSpent: String, // consider migrating to a value type
    pub timeSpentSeconds: i32,
    pub issueId: i32, // Numeric FK to issue
    pub comment: Option<String>,
}

impl LocalWorklog {
    /// Converts a Jira `Worklog` entry into a `LocalWorklog` entry.
    ///
    /// # Arguments
    ///
    /// * `worklog` - A reference to a `Worklog` object from Jira that needs to be converted.
    /// * `issue_key` - A reference to the `IssueKey` associated with the worklog entry.
    ///
    /// # Returns
    ///
    /// Returns a new `LocalWorklog` instance containing the converted data.
    ///
    /// # Panics
    ///
    /// This function will panic if `worklog.issueId` cannot be parsed into an `i32`.
    #[must_use]
    pub fn from_worklog(worklog: &Worklog, issue_key: &IssueKey) -> Self {
        LocalWorklog {
            issue_key: issue_key.clone(),
            id: worklog.id.clone(),
            author: worklog.author.displayName.clone(),
            created: worklog.created.with_timezone(&Local),
            updated: worklog.updated.with_timezone(&Local),
            started: worklog.started.with_timezone(&Local),
            timeSpent: worklog.timeSpent.clone(),
            timeSpentSeconds: worklog.timeSpentSeconds,
            issueId: worklog.issueId.parse().unwrap(),
            comment: worklog.comment.clone(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, Clone)]
pub struct JiraIssueInfo {
    pub issue_key: IssueKey,
    pub summary: String,
}

/// Represents a timer record in the database
///
/// Each timer is associated with an issue and tracks a time period
/// with start and optional end timestamps. Timers without an end time
/// are considered active.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Timer {
    /// Unique identifier for the timer, auto-assigned by the database
    pub id: Option<i64>,

    /// Foreign key to the associated issue
    pub issue_key: String,

    /// When this timer record was created
    pub created_at: DateTime<Local>,

    /// When the timer was started
    pub started_at: DateTime<Local>,

    /// When the timer was stopped (null for active timers)
    pub stopped_at: Option<DateTime<Local>>,

    /// Whether this timer has been synchronized with a remote system
    pub synced: bool,

    /// Optional comment about the work being tracked
    pub comment: Option<String>,
}

impl Timer {
    /// Creates a new timer for the specified issue that starts now
    #[must_use]
    pub fn start_new(issue_id: String) -> Self {
        let now = Utc::now();
        Self {
            id: None,
            issue_key: issue_id,
            created_at: now.with_timezone(&Local),
            started_at: now.with_timezone(&Local),
            stopped_at: None,
            synced: false,
            comment: None,
        }
    }

    /// Checks if this timer is currently active (not stopped)
    #[must_use]
    pub fn is_active(&self) -> bool {
        self.stopped_at.is_none()
    }

    /// Gets the duration of this timer if it has been stopped
    #[must_use]
    pub fn duration(&self) -> Option<chrono::Duration> {
        self.stopped_at.map(|end| end - self.started_at)
    }

    /// Stops this timer at the current time
    pub fn stop(&mut self) {
        if self.is_active() {
            self.stopped_at = Some(Utc::now().with_timezone(&Local));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Local;
    use jira::models::core::IssueKey;

    #[test]
    fn test_timer_start_new() {
        let timer = Timer::start_new("TEST-123".to_string());

        assert!(timer.id.is_none());
        assert_eq!(timer.issue_key, "TEST-123");
        assert!(timer.is_active());
        assert!(!timer.synced);
        assert!(timer.comment.is_none());
        assert!(timer.duration().is_none()); // Active timer has no duration
    }

    #[test]
    fn test_timer_is_active() {
        let mut timer = Timer::start_new("TEST-123".to_string());

        // Initially active
        assert!(timer.is_active());

        // Stop the timer
        timer.stop();
        assert!(!timer.is_active());
        assert!(timer.stopped_at.is_some());
    }

    #[test]
    fn test_timer_stop() {
        let mut timer = Timer::start_new("TEST-123".to_string());
        let start_time = timer.started_at;

        // Timer should be active initially
        assert!(timer.is_active());
        assert!(timer.stopped_at.is_none());

        // Stop the timer
        timer.stop();

        // Timer should no longer be active
        assert!(!timer.is_active());
        assert!(timer.stopped_at.is_some());

        // Stopped time should be after start time
        if let Some(stopped_at) = timer.stopped_at {
            assert!(stopped_at >= start_time);
        }
    }

    #[test]
    fn test_timer_stop_already_stopped() {
        let mut timer = Timer::start_new("TEST-123".to_string());

        // Stop the timer
        timer.stop();
        let first_stop_time = timer.stopped_at;

        // Stop again - should not change the stop time
        timer.stop();
        assert_eq!(timer.stopped_at, first_stop_time);
    }

    #[test]
    fn test_timer_duration() {
        let start_time = Local::now();
        let stop_time = start_time + chrono::Duration::hours(2);

        let timer = Timer {
            id: Some(1),
            issue_key: "TEST-123".to_string(),
            created_at: start_time,
            started_at: start_time,
            stopped_at: Some(stop_time),
            synced: false,
            comment: None,
        };

        let duration = timer.duration().unwrap();
        assert_eq!(duration.num_hours(), 2);
    }

    #[test]
    fn test_timer_duration_active() {
        let timer = Timer::start_new("TEST-123".to_string());

        // Active timer should have no duration
        assert!(timer.duration().is_none());
    }

    #[test]
    fn test_local_worklog_from_worklog() {
        use chrono::Utc;
        use jira::models::core::Author;
        use jira::models::worklog::Worklog;

        let author = Author {
            accountId: "acc123".to_string(),
            emailAddress: Some("test@example.com".to_string()),
            displayName: "Test User".to_string(),
        };

        let worklog = Worklog {
            id: "456".to_string(),
            author,
            comment: Some("Test comment".to_string()),
            created: Utc::now(),
            updated: Utc::now(),
            started: Utc::now(),
            timeSpent: "1h".to_string(),
            timeSpentSeconds: 3600,
            issueId: "12345".to_string(),
        };

        let issue_key = IssueKey::from("TEST-123");
        let local_worklog = LocalWorklog::from_worklog(&worklog, &issue_key);

        assert_eq!(local_worklog.issue_key, issue_key);
        assert_eq!(local_worklog.id, "456");
        assert_eq!(local_worklog.author, "Test User");
        assert_eq!(local_worklog.timeSpent, "1h");
        assert_eq!(local_worklog.timeSpentSeconds, 3600);
        assert_eq!(local_worklog.issueId, 12345);
        assert_eq!(local_worklog.comment, Some("Test comment".to_string()));
    }

    #[test]
    fn test_jira_issue_info_creation() {
        let issue_info = JiraIssueInfo {
            issue_key: IssueKey::from("PROJ-456"),
            summary: "Test issue summary".to_string(),
        };

        assert_eq!(issue_info.issue_key.value(), "PROJ-456");
        assert_eq!(issue_info.summary, "Test issue summary");
    }

    #[test]
    fn test_timer_with_comment() {
        let mut timer = Timer::start_new("TEST-123".to_string());
        timer.comment = Some("Working on feature X".to_string());

        assert_eq!(timer.comment, Some("Working on feature X".to_string()));
    }

    #[test]
    fn test_timer_sync_flag() {
        let mut timer = Timer::start_new("TEST-123".to_string());

        // Initially not synced
        assert!(!timer.synced);

        // Mark as synced
        timer.synced = true;
        assert!(timer.synced);
    }

    #[test]
    fn test_timer_with_id() {
        let mut timer = Timer::start_new("TEST-123".to_string());
        timer.id = Some(42);

        assert_eq!(timer.id, Some(42));
    }
}
