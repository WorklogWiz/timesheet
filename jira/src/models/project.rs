use serde::{Deserialize, Serialize};

use super::issue::Issue;

#[derive(Debug, Deserialize, Serialize)]
pub struct JiraProjectsPage {
    #[serde(alias = "nextPage")]
    pub next_page: Option<String>,
    #[serde(alias = "startAt")]
    pub start_at: i32,
    #[serde(alias = "maxResults")]
    pub max_results: i32,
    pub total: Option<i32>,
    #[serde(alias = "isLast")]
    pub is_last: Option<bool>,
    pub values: Vec<Project>,
}

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct Project {
    /// Unique numeric identity of a jira project
    pub id: String,
    /// The jira project key, typically a short upper-case abbreviation
    pub key: String,
    /// The name of the jira project
    pub name: String,
    #[serde(alias = "self")]
    pub url: String,
    #[serde(alias = "isPrivate")]
    pub is_private: bool,
    #[serde(skip)] // Added after Deserializing
    /// Collection of issues belonging to the jira project
    pub issues: Vec<Issue>,
}
