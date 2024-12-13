use serde::{Deserialize, Serialize};

use super::{
    core::{JiraFields, JiraKey},
    worklog::Worklog,
};

/// Represents a page of Jira issues retrieved from Jira
#[derive(Debug, Deserialize, Serialize)]
#[allow(non_snake_case)]
pub struct IssuesPage {
    #[serde(alias = "startAt")]
    pub start_at: i32,
    #[serde(alias = "maxResults")]
    pub max_results: i32,
    pub total: Option<i32>,
    pub isLast: Option<bool>,
    pub issues: Vec<Issue>,
}

/// Represents a jira issue
#[derive(Clone, Debug, Deserialize, Serialize, Default)]
pub struct Issue {
    /// Numeric id of the jira issue
    pub id: String, // Numeric id
    #[serde(alias = "self")]
    pub self_url: String,
    /// The key of the jira issue, typically used and referenced by the user.
    pub key: JiraKey,

    /// Holds the work logs after deserializing them from Jira
    #[serde(skip)] // Added after deserializing
    pub worklogs: Vec<Worklog>,
    pub fields: JiraFields,
}
