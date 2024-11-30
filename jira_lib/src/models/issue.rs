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
#[derive(Debug, Deserialize, Serialize, Default)]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_to_jira_issue() {
        let json_data = include_str!("../../tests/issue_time_63.json");
        let _jira_issue: Issue = serde_json::from_str(json_data).unwrap();
    }
}
