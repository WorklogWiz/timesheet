use serde::{Deserialize, Serialize};

use super::issue::Issue;

#[derive(Debug, Deserialize, Serialize)]
#[allow(non_snake_case)]
pub struct JiraProjectsPage {
    pub nextPage: Option<String>,
    pub startAt: i32,
    pub maxResults: i32,
    pub total: Option<i32>,
    pub isLast: Option<bool>,
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
