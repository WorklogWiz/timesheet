use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use jira::models::core::IssueKey;
use jira::models::worklog::Worklog;

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
    pub issueId: String, // Numeric FK to issue
    pub comment: Option<String>,
}

impl LocalWorklog {
    /// Converts a Jira `Worklog` entry into a `LocalWorklog` entry
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
            issueId: worklog.issueId.clone(),
            comment: worklog.comment.clone(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, Clone)]
pub struct JiraIssueInfo {
    pub issue_key: IssueKey,
    pub summary: String,
}