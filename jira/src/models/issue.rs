use super::{
    core::{JiraFields, JiraKey},
    worklog::Worklog,
};
use crate::models::project::JiraProjectKey;
use serde::{Deserialize, Serialize};

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

#[derive(Serialize, Deserialize, Debug)]
pub struct JiraNewIssueResponse {
    pub id: String,
    pub key: String,
}

#[allow(clippy::module_name_repetitions)]
#[derive(Serialize, Debug)]
pub struct JiraNewIssue {
    pub fields: JiraIssueFields,
}

#[derive(Serialize, Debug)]
pub struct JiraIssueFields {
    pub project: JiraProjectKey,
    pub issuetype: JiraIssueType,
    pub summary: String,
    pub description: Option<String>,
}

#[derive(Serialize, Debug)]
pub struct JiraIssueType {
    pub name: String,
}
