use chrono::Utc;
use chrono::{DateTime, Local};
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
