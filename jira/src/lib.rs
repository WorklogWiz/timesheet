//!
//! `jira_lib` is a collection of useful functions when interacting with
//! Jira using the official REST interface.
//!
//! Many of the types have been declared specifically for the purpose of work log management,
//! and are hence not generic.
use std::{collections::BTreeMap, error::Error, fmt::{self, Formatter}};

use chrono::{DateTime, Days, Local, NaiveDateTime, TimeZone};
use futures::StreamExt;
use log::{debug, info};
use reqwest::{header::{ACCEPT, CONTENT_TYPE}, Client, Method, RequestBuilder, StatusCode};
use config::JiraClientConfiguration;
use models::{
    issue::{Issue, IssuesPage},
    project::{JiraProjectsPage, Project},
    user::User,
    worklog::{Insert, Worklog, WorklogsPage},
};

use serde::{de::DeserializeOwned, Deserialize, Serialize};
use url::{ParseError, Url};

pub mod config;
pub mod models;

type Result<T> = std::result::Result<T, JiraError>;

const FUTURE_BUFFER_SIZE: usize = 20;

#[derive(Serialize, Deserialize, Debug)]
pub struct Errors {
    #[serde(rename = "errorMessages")]
    pub error_messages: Vec<String>,
    pub errors: BTreeMap<String, String>,
}

#[cfg_attr(doc, aquamarine::aquamarine)]
///
/// ```mermaid
/// graph LR
///     s([Source]) --> a[[aquamarine]]
///      r[[rustdoc]] --> f([Docs w/ Mermaid!])
///      subgraph rustc[Rust Compiler]
///      a -. inject mermaid.js .-> r
///      end
/// ```
#[derive(Debug)]
pub enum JiraError {
    Unauthorized,
    MethodNotAllowed,
    NotFound(String),
    Fault { code: StatusCode, errors: Errors },
    RequiredParameter(String),
    DeleteFailed(StatusCode),
    WorklogNotFound(String, String),
    RequestError(reqwest::Error),
    SerializationError(serde_json::error::Error),
    ParseError(ParseError),
    UnexpectedStatus,
}

#[allow(clippy::enum_glob_use)]
impl fmt::Display for JiraError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        use crate::JiraError::*;

        match self {
            RequiredParameter(param_name) => writeln!(f, "Parameter '{param_name}' must contain a a value"),
            DeleteFailed(sc) => writeln!(f, "Failed to delete: {sc}"),
            WorklogNotFound(issue, worklog_id) => writeln!(f, "Worklog entry with issue_key: {issue} and worklog_id: {worklog_id} not found"),
            RequestError(e) => writeln!(f, "Internal error in reqwest library: {}", e.to_string().as_str()),
            ParseError(e) =>  writeln!(f, "Could not connect to Jira: {e:?}!"),
            SerializationError(e) => writeln!(f, "Could not serialize/deserialize: {e:?}!"),
            Fault {
                ref code,
                ref errors,
            } => writeln!(f, "Jira Client Error ({code}):\n{errors:#?}"),
            Unauthorized => todo!(),
            MethodNotAllowed => todo!(),
            NotFound(url) => writeln!(f, "Not found: '{url}'"),
            UnexpectedStatus => todo!(),
        }
    }
}

impl Error for JiraError {
    // Ref: https://stackoverflow.com/questions/62869360/should-an-error-with-a-source-include-that-source-in-the-display-output
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            JiraError::RequiredParameter(_) => None,
            _ => self.source(),
        }
    }
}

impl From<ParseError> for JiraError {
    fn from(error: ParseError) -> JiraError {
        JiraError::ParseError(error)
    }
}

impl From<reqwest::Error> for JiraError {
    fn from(error: reqwest::Error) -> JiraError {
        JiraError::RequestError(error)
    }
}

impl From<serde_json::error::Error> for JiraError {
    fn from(error: serde_json::error::Error) -> JiraError {
        JiraError::SerializationError(error)
    }
}

#[derive(Clone, Debug)]
pub enum Credentials {
    Anonymous,
    Basic(String, String),
    Bearer(String),
}

impl Credentials {
    fn apply(&self, request: RequestBuilder) -> RequestBuilder {
        match self {
            Credentials::Anonymous => request,
            Credentials::Basic(ref user, ref pass) => {
                request.basic_auth(user.to_owned(), Some(pass.to_owned()))
            }
            Credentials::Bearer(ref token) => request.bearer_auth(token.to_owned()),
        }
    }
}

#[derive(Clone)]
pub struct Jira {
    host: Url,
    api: String,
    credentials: Credentials,
    pub client: Client,
}

impl Jira {
    #[allow(clippy::missing_errors_doc)]
    pub fn new<H>(host: H, credentials: Credentials) -> Result<Jira>
    where
    H: Into<String>,
    {
        let host = Url::parse(&host.into())?;

        Ok(Jira {
            host,
            api: "api".to_string(),
            client: Client::new(),
            credentials,
        })
    }

    #[allow(clippy::missing_errors_doc)]
    #[allow(clippy::missing_panics_doc)]
    pub fn from(cfg: &JiraClientConfiguration) -> Result<Jira> {
        let url = Url::parse(&cfg.jira_url)?;
        Jira::new(
            format!("{}://{}", url.scheme(), url.host().unwrap()),
            Credentials::Basic(cfg.user.clone(), cfg.token.clone()))
    }

    async fn request<D>(
        &self,
        method: Method,
        endpoint: &str,
        body: Option<Vec<u8>>,
    ) -> Result<D>
    where
        D: DeserializeOwned,
    {
        let url = self.host
            .join(&format!("rest/{}/latest{endpoint}", self.api))?;

        let mut request = self
            .client
            .request(method, url.clone())
            .header(CONTENT_TYPE, "application/json")
            .header(ACCEPT, "application/json");

            request = self.credentials.apply(request);

        if let Some(body) = body {
            request = request.body(body);
        }
        debug!("request '{:?}'", request);

        let response = request.send().await?;

        let status = response.status();
        let body = &response.text().await?;
        debug!("status {:?} body '{:?}'", status, body);
        match status {
            StatusCode::UNAUTHORIZED => Err(JiraError::Unauthorized),
            StatusCode::METHOD_NOT_ALLOWED => Err(JiraError::MethodNotAllowed),
            StatusCode::NOT_FOUND => Err(JiraError::NotFound(url.to_string())),
            client_err if client_err.is_client_error() => Err(JiraError::Fault {
                code: status,
                errors: serde_json::from_str::<Errors>(body)?,
            }),
            _ => {
                let data = if body.is_empty() { "null" } else { body };
                Ok(serde_json::from_str::<D>(data)?)
            }
        }
    }

    #[allow(clippy::missing_errors_doc)]
    pub async fn get<D>(&self, endpoint: &str) -> Result<D>
    where
        D: DeserializeOwned,
    {
        self.request::<D>(Method::GET, endpoint, None).await
    }

    async fn delete<D>(&self, endpoint: &str) -> Result<D>
    where
        D: DeserializeOwned,
    {
        self.request::<D>(Method::DELETE, endpoint, None).await
    }

    async fn post<D, S>(&self, endpoint: &str, body: S) -> Result<D>
    where
        D: DeserializeOwned,
        S: Serialize,
    {
        let data = serde_json::to_string::<S>(&body)?;
        self.request::<D>(Method::POST, endpoint, Some(data.into_bytes())).await
    }

    /*
    async fn put<D, S>(&self, endpoint: &str, body: S) -> Result<D>
    where
        D: DeserializeOwned,
        S: Serialize,
    {
        let data = serde_json::to_string::<S>(&body)?;
        debug!("Json request: {}", data);
        self.request::<D>(Method::PUT, endpoint, Some(data.into_bytes())).await
    }

    #[allow(clippy::missing_errors_doc)]
    async fn get_issue_by_id_or_key(&self, id: &str) -> Result<Issue> {

        self.get::<Issue>(&format!("/issue/{id}")).await
    }

    */

    /// Retrieves all Jira projects, filtering out the private ones
    /// Only used in examples
    #[allow(clippy::missing_errors_doc)]
    pub async fn get_projects(&self, project_keys: Vec<String>) -> Result<Vec<Project>> {
        let start_at = 0;

        // Retrieves first page of Jira projects
        let mut project_page = self
            .get::<JiraProjectsPage>(&Self::project_search_resource(start_at, project_keys))
            .await?;

        let mut projects = Vec::<Project>::new();
        if project_page.values.is_empty() {
            // No projects, just return empty vector
            return Ok(projects);
        }

        projects.append(
            &mut project_page
                .values
                .into_iter()
                .filter(|p| !p.is_private)
                .collect(),
        );

        // While there is a URL for the next page ...
        while let Some(url) = &project_page.next_page {
            // Fetch next page of data
            project_page = self.get::<JiraProjectsPage>(&url.clone()).await?;
            // Filter out the private projects and append to our list of projects
            projects.append(
                &mut project_page
                    .values
                    .into_iter()
                    .filter(|p| !p.is_private)
                    .collect(),
            );
        }
        Ok(projects)
    }

    #[allow(
        clippy::cast_sign_loss,
        clippy::cast_possible_truncation,
        clippy::cast_possible_wrap,
        clippy::missing_errors_doc
    )]
    pub async fn get_issues_for_project(
        &self,
        project_key: String,
    ) -> Result<Vec<Issue>> {
        let mut resource = Self::compose_resource_and_params(&project_key, 0, 1024);

        let mut issues = Vec::<Issue>::new();
        loop {
            let mut issue_page = self
                .get::<IssuesPage>(&resource)
                .await?;

            // issues.len() will be invalid once we move the contents of the issues into our result
            let is_last_page = issue_page.issues.len() < issue_page.max_results as usize;
            if !is_last_page {
                resource = Self::compose_resource_and_params(
                    &project_key,
                    issue_page.start_at + issue_page.issues.len() as i32,
                    issue_page.max_results,
                );
            }
            issues.append(&mut issue_page.issues);
            if is_last_page {
                break;
            }
        }
        Ok(issues)
    }

    /// Only used in examples
    #[allow(clippy::missing_errors_doc)]
    #[allow(
        clippy::cast_sign_loss,
        clippy::cast_possible_truncation,
        clippy::cast_possible_wrap
    )]
    pub async fn get_worklogs_for(
        &self,
        issue_key: String,
        started_after: NaiveDateTime,
    ) -> Result<Vec<Worklog>> {
        let mut resource_name =
            Self::compose_worklogs_url(issue_key.as_str(), 0, 5000, started_after);
        let mut worklogs: Vec<Worklog> = Vec::<Worklog>::new();

        debug!("Retrieving worklogs for {}", issue_key);
        loop {
            let mut worklog_page = self
                .get::<WorklogsPage>(&resource_name)
                .await?;
            let is_last_page = worklog_page.worklogs.len() < worklog_page.max_results as usize;
            if !is_last_page {
                resource_name = Self::compose_worklogs_url(
                    issue_key.as_str(),
                    worklog_page.startAt + worklog_page.worklogs.len() as i32,
                    worklog_page.max_results,
                    started_after,
                );
            }
            worklogs.append(&mut worklog_page.worklogs);
            if is_last_page {
                break;
            }
        }
        Ok(worklogs)
    }

    // -----------------------
    // Static methods
    fn project_search_resource(start_at: i32, project_keys: Vec<String>) -> String {
        // Seems 50 is the max value of maxResults
        let mut resource = format!("/project/search?maxResults=50&startAt={start_at}");
        if !project_keys.is_empty() {
            for key in project_keys {
                resource.push_str("&keys=");
                resource.push_str(key.as_str());
            }
        }
        resource
    }

    fn compose_worklogs_url(
        issue_key: &str,
        start_at: i32,
        max_results: i32,
        started_after: NaiveDateTime,
    ) -> String {
        format!(
            "/issue/{}/worklog?startAt={}&maxResults={}&startedAfter={}",
            issue_key,
            start_at,
            max_results,
            Local.from_utc_datetime(&started_after).timestamp_millis()
        )
    }

    fn compose_resource_and_params(project_key: &str, start_at: i32, max_results: i32) -> String {
        let jql = format!("project=\"{project_key}\" and resolution=Unresolved");
        let jql_encoded = urlencoding::encode(&jql);

        // Custom field customfield_10904 is the project asset custom field
        let resource = format!(
            "/search?jql={}&startAt={}&maxResults={}&fields={}",
            jql_encoded, start_at, max_results, "summary,customfield_10904"
        );
        resource
    }

    #[allow(dead_code)]
    #[allow(clippy::missing_errors_doc)]
    async fn get_issues_for_projects(self, projects: Vec<Project>) -> Result<Vec<Project>> {
        let mut futures_stream = futures::stream::iter(projects)
            .map(|mut project| {
                let me = self.clone();
                tokio::spawn(async move {
                    let Ok(issues) = me.get_issues_for_project(project.key.clone()).await
                    else {
                        todo!()
                    };
                    let _old = std::mem::replace(&mut project.issues, issues);
                    project
                })
            })
            .buffer_unordered(FUTURE_BUFFER_SIZE);

        let mut result = Vec::<Project>::new();
        while let Some(r) = futures_stream.next().await {
            match r {
                Ok(jp) => result.push(jp),
                Err(e) => eprintln!("Error: {e:?}"),
            }
        }
        Ok(result)
    }

    // todo: clean up and make simpler!
    #[allow(dead_code)]
    #[allow(clippy::missing_errors_doc, clippy::missing_panics_doc)]
    async fn get_issues_and_worklogs(
        self,
        projects: Vec<Project>,
        issues_filter: Vec<String>,
        started_after: NaiveDateTime,
    ) -> Result<Vec<Project>> {
        let mut futures_stream = futures::stream::iter(projects)
            .map(|mut project| {
                debug!(
                    "Creating future for {}, filters={:?}",
                    &project.key, issues_filter
                );

                let filter = issues_filter.clone();
                let me = self.clone(); // Clones the vector to allow async move
                tokio::spawn(async move {
                    let issues =
                        me.get_issues_for_project(project.key.clone())
                            .await?;
                    debug!(
                        "Extracted {} issues. Applying filter {:?}",
                        issues.len(),
                        filter
                    );
                    let issues: Vec<Issue> = issues
                        .into_iter()
                        .filter(|issue| {
                            filter.is_empty()
                                || !filter.is_empty() && filter.contains(&issue.key.value)
                        })
                        .collect();
                    debug!("Filtered {} issues for {}", issues.len(), &project.key);
                    let _old = std::mem::replace(&mut project.issues, issues);
                    for issue in &mut project.issues {
                        debug!("Retrieving worklogs for issue {}", &issue.key);
                        let key = issue.key.to_string();
                        let mut worklogs =
                            match me.get_worklogs_for(key, started_after).await {
                                Ok(result) => result,
                                Err(e) => return Err(e),
                            };
                        debug!(
                            "Issue {} has {} worklog entries",
                            issue.key.value,
                            worklogs.len()
                        );
                        issue.worklogs.append(&mut worklogs);
                    }
                    Ok(project)
                })
            })
            .buffer_unordered(FUTURE_BUFFER_SIZE);

        let mut result = Vec::<Project>::new();
        while let Some(r) = futures_stream.next().await {
            match r.unwrap() {
                Ok(jp) => {
                    info!("Data retrieved from Jira for project {}", jp.key);
                    result.push(jp);
                }
                Err(e) => eprintln!("Error: {e:?}"),
            }
        }
        Ok(result)
    }

    #[allow(
        clippy::missing_errors_doc,
        clippy::missing_panics_doc
    )]
    pub async fn insert_worklog(
        &self,
        issue_id: &str,
        started: DateTime<Local>,
        time_spent_seconds: i32,
        comment: &str,
    ) -> Result<Worklog> {
        // This is how Jira needs it.
        // Note! The formatting in Jira is based on the time zone of the user. Remember to change it
        // if you fly across the ocean :-)
        // Move this into a function
        let start = started.format("%Y-%m-%dT%H:%M:%S.%3f%z");
        let worklog_entry = Insert {
            timeSpentSeconds: time_spent_seconds,
            comment: comment.to_string(),
            started: start.to_string(),
        };

        let url = format!("/issue/{issue_id}/worklog");
        self.post::<Worklog, Insert>(&url, worklog_entry).await
    }

    #[allow(clippy::missing_errors_doc)]
    pub async fn delete_worklog(
        &self,
        issue_id: String,
        worklog_id: String,
    ) -> Result<()> {
        let url = format!("/issue/{}/worklog/{}", &issue_id, &worklog_id);
        let _ = self.delete::<Option<Worklog>>(&url).await?;
        Ok(())
    }

    #[allow(clippy::missing_errors_doc)]
    pub async fn get_current_user(&self) -> Result<User> {
        self.get::<User>("/myself").await
    }

    #[allow(clippy::missing_errors_doc)]
    pub async fn get_worklog(&self, issue_id: &str, worklog_id: &str) -> Result<Worklog> {
        let resource = format!("/issue/{issue_id}/worklog/{worklog_id}");
        self.get::<Worklog>(&resource).await
    }

    #[allow(
        clippy::missing_errors_doc,
        clippy::missing_panics_doc
    )]
    pub async fn get_worklogs_for_current_user(
        &self,
        issue_key: &str,
        started_after: Option<DateTime<Local>>,
    ) -> Result<Vec<Worklog>> {
        assert!(!issue_key.is_empty(), "Must specify an issue key");
        let current_user = self.get_current_user().await?;
        let date_time = started_after.unwrap_or_else(|| {
            // Defaults to a month (approx)
            Local::now().checked_sub_days(Days::new(30)).unwrap()
        });
        let naive_date_time = DateTime::from_timestamp_millis(date_time.timestamp_millis())
            .unwrap()
            .naive_local();
        let result =
            self.get_worklogs_for(issue_key.to_string(), naive_date_time)
                .await?;
        debug!("Work logs retrieved, filtering them for current user ....");
        Ok(result
            .into_iter()
            .filter(|wl| wl.author.accountId == current_user.account_id)
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Server;

    #[tokio::test]
    async fn fetch_myself_success() -> Result<()>{
        let mut server = Server::new_async().await;
        let url = server.url();
        let _m = server.mock("GET", "/rest/api/latest/myself")
            .with_status(200)
            .with_body(r#"{
                "self": "foo",
                "accountId": "foo",
                "emailAddress": "foo@bar.com",
                "displayName": "foo",
                "timeZone": "local"
            }"#)
            .create_async()
            .await;

        let client = Jira::new(url, Credentials::Basic("foo@bar.com".to_string(), String::new()))?;
        let user = client.get_current_user().await?;

        assert_eq!(user.email_address, "foo@bar.com");
        Ok(())
    }

    #[tokio::test]
    async fn fetch_myself_unauth() -> Result<()>{
        let mut server = Server::new_async().await;
        let url = server.url();
        let _m = server.mock("GET", "/rest/api/latest/myself")
            .with_status(403)
            .with_body(r#"{
                "errorMessages": ["foo"],
                "errors": {}
            }"#)
            .create_async()
            .await;

        let client = Jira::new(url, Credentials::Basic("foo@bar.com".to_string(), String::new()))?;
        if let Err(unauth) = client.get_current_user().await {
            #[allow(clippy::single_match_else)]
            match unauth {
                JiraError::Fault { code, errors } => {
                    assert_eq!(code, 403);
                    assert_eq!(errors.error_messages[0], "foo");
                }
                _ => panic!(),
            }
        } else {
            panic!("Expected an error")
        };

        Ok(())
    }
}
