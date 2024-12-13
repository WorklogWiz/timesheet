use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::core::Author;

#[derive(Debug, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct WorklogsPage {
    pub startAt: i32,
    #[serde(alias = "maxResults")]
    pub max_results: i32,
    pub total: i32,
    pub worklogs: Vec<Worklog>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq, PartialOrd)]
#[allow(non_snake_case)]
pub struct Worklog {
    pub id: String,
    // "557058:189520f0-d1fb-4a0d-b555-bc44ec1f4ebc"
    pub author: Author,
    pub created: DateTime<Utc>,
    pub updated: DateTime<Utc>,
    pub started: DateTime<Utc>,
    pub timeSpent: String,
    pub timeSpentSeconds: i32,
    pub issueId: String, // Numeric FK to issue
    pub comment: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct Insert {
    pub comment: String,
    pub started: String,
    pub timeSpentSeconds: i32,
}
