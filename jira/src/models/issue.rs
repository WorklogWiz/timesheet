use super::{core::IssueKey, worklog::Worklog};
use crate::models::core::Fields;
use crate::models::project::JiraProjectKey;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Deserializer, Serialize};

/// Holds Responses from Jira when performing JQL queries, which
/// will always return a collection of Issues with potential sub structures.
#[derive(Debug, Serialize)]
pub struct IssuesResponse<T>
where
    T: DeserializeOwned,
{
    pub issues: Vec<T>,
    #[serde(rename = "nextPageToken")] // Ensure field matches the JSON representation
    pub next_page_token: Option<String>,
}

impl<T> IssuesResponse<T> where T: DeserializeOwned {}

// Manually implement `Deserialize` for `IssuesResponse<T>` to handle
// Deserialization of whatever is contained in `issues`
impl<'de, T> Deserialize<'de> for IssuesResponse<T>
where
    T: DeserializeOwned,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct InternalIssuesResponse<T> {
            issues: Vec<T>,
            #[serde(rename = "nextPageToken")]
            next_page_token: Option<String>,
        }

        let internal = InternalIssuesResponse::deserialize(deserializer)?;
        Ok(IssuesResponse {
            issues: internal.issues,
            next_page_token: internal.next_page_token,
        })
    }
}

#[derive(Debug, Serialize, Deserialize, PartialOrd, PartialEq, Ord, Eq)]
#[allow(clippy::module_name_repetitions)]
pub struct IssueSummary {
    pub id: String,
    pub key: IssueKey, // TODO: Add components
    pub fields: Fields,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Worklogs {
    pub worklogs: Vec<Worklog>,
}

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
    pub key: IssueKey,

    pub fields: Fields,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct NewIssueResponse {
    pub id: String,
    pub key: IssueKey,
}

#[allow(clippy::module_name_repetitions)]
#[derive(Serialize, Debug)]
pub struct NewIssue {
    pub fields: NewIssueFields,
}

#[allow(clippy::module_name_repetitions)]
#[derive(Serialize, Debug)]
pub struct NewIssueFields {
    pub project: JiraProjectKey,
    pub issuetype: IssueType,
    pub summary: String,
    pub description: Option<String>,
    pub components: Vec<ComponentId>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ComponentId {
    pub id: String,
}
#[allow(clippy::module_name_repetitions)]
#[derive(Serialize, Debug)]
pub struct IssueType {
    pub name: String,
}
