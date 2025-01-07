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
    pub timeSpent: String, // consider migrating to value type
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
