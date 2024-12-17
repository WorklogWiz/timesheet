//!
//! `jira_lib` is a collection of useful functions when interacting with
//! Jira using the official REST interface.
//!
//! Many of the types have been declared specifically for the purpose of work log management,
//! and are hence not generic.
use std::{
    collections::BTreeMap,
    error::Error,
    fmt::{self, Formatter},
};

use chrono::{DateTime, Days, Local, NaiveDateTime, TimeZone};
use futures::StreamExt;
use log::{debug, info, warn};
use models::{
    issue::{Issue, IssuesPage},
    project::{JiraProjectsPage, Project},
    user::User,
    worklog::{Insert, Worklog, WorklogsPage},
};
use reqwest::{
    header::{ACCEPT, CONTENT_TYPE},
    Client, Method, RequestBuilder, StatusCode,
};

use crate::models::core::JiraKey;
use crate::models::issue::{JiraIssueFields, JiraIssueType, JiraNewIssue, JiraNewIssueResponse};
use crate::models::project::JiraProjectKey;
use crate::models::setting::{GlobalSettings, TimeTrackingConfiguration};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use url::{ParseError, Url};

pub mod models;

type Result<T> = std::result::Result<T, JiraError>;

const FUTURE_BUFFER_SIZE: usize = 20;

#[derive(Serialize, Deserialize, Debug)]
pub struct Errors {
    #[serde(rename = "errorMessages")]
    pub error_messages: Vec<String>,
    pub errors: Option<BTreeMap<String, String>>,
}

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
            RequiredParameter(param_name) => {
                writeln!(f, "Parameter '{param_name}' must contain a a value")
            }
            DeleteFailed(sc) => writeln!(f, "Failed to delete: {sc}"),
            WorklogNotFound(issue, worklog_id) => writeln!(
                f,
                "Worklog entry with issue_key: {issue} and worklog_id: {worklog_id} not found"
            ),
            RequestError(e) => writeln!(
                f,
                "Internal error in reqwest library: {}",
                e.to_string().as_str()
            ),
            ParseError(e) => writeln!(f, "Could not connect to Jira: {e:?}!"),
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

///
/// # Example
///
/// ```rust,ignore
/// use jira::Jira;
/// use jira::Credentials;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let jira = Jira::new(
///         "https://your-jira-instance.atlassian.net",
///         Credentials::Basic("your_username".to_string(), "your_api_token".to_string()),
///     )?;
///
///     let response: serde_json::Value = jira.get("/issue/TEST-1").await?;
///     println!("Issue data: {:?}", response);
///
///     Ok(())
/// }
/// ```
#[derive(Clone)]
pub struct Jira {
    host: Url,
    api: String,
    credentials: Credentials,
    pub client: Client,
}

impl Jira {
    /// Example usage:
    ///
    /// ```rust,ignore
    /// use jira::Jira;
    /// use jira::Credentials;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let jira = Jira::new(
    ///         "https://your-jira-instance.atlassian.net",
    ///         Credentials::Basic("your_username".to_string(), "your_api_token".to_string()),
    ///     )?;
    ///
    ///     let response: serde_json::Value = jira.get("/issue/TEST-1").await?;
    ///     println!("Issue data: {:?}", response);
    ///
    ///     Ok(())
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// This function returns an error in the following cases:
    ///
    /// * If `host` is not a valid URL, a `ParseError` will be returned.
    /// * If the provided credentials are not valid or cause unexpected behavior during API interaction,
    ///   subsequent requests using this instance may fail.
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

    async fn request<D>(&self, method: Method, endpoint: &str, body: Option<Vec<u8>>) -> Result<D>
    where
        D: DeserializeOwned,
    {
        let url = self
            .host
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

    ///
    /// Sends an HTTP GET request to the specified Jira endpoint.
    ///
    /// # Parameters
    ///
    /// * `endpoint`: A string slice that specifies the Jira API endpoint to be called.
    ///
    /// # Returns
    ///
    /// A `Result` containing the response of type `D` if successful, or a `JiraError` if an error occurs.
    ///
    /// # Errors
    ///
    /// This function returns errors for the following cases:
    /// * Failure in sending the GET request.
    /// * Issues while deserializing the response into the expected type `D`.
    ///
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
        self.request::<D>(Method::POST, endpoint, Some(data.into_bytes()))
            .await
    }

    /// Searches for Jira issues based on provided projects and/or issue keys.
    ///
    /// # Parameters
    /// * `projects`: A vector of project keys (e.g., `["TEST", "PROJ"]`). Can be empty.
    /// * `issue_keys`: A slice of issue keys to search for (e.g., `["TEST-1", "PROJ-2"]`). Can be empty.
    ///
    /// # Returns
    /// A `Result` containing a vector of `Issue` if successful, or a `JiraError` if an error occurs.
    ///
    /// # Errors
    /// Returns an error if:
    /// * Both `projects` and `issue_keys` are empty.
    /// * Network requests fail.
    /// * Parsing the response fails.
    ///
    /// # Examples
    /// This function requires proper setup of Jira client and works asynchronously.
    pub async fn search_issues(
        &self,
        projects: &Vec<&str>,
        issue_keys: &[JiraKey],
    ) -> Result<Vec<Issue>> {
        if projects.is_empty() && issue_keys.is_empty() {
            warn!("No projects or issue keys provided");
            return Ok(Vec::<Issue>::new());
        }

        let mut jql = String::new();
        if !projects.is_empty() {
            jql = format!("project in ({})", projects.join(","));
        }
        if !issue_keys.is_empty() {
            // and comma-separated list of issue keys
            let keys_spec = issue_keys
                .iter()
                .map(std::string::ToString::to_string)
                .collect::<Vec<_>>()
                .join(",");

            if jql.is_empty() {
                // No Project clause, so only add the issue keys
                jql.push_str(format!("issuekey in ({keys_spec})").as_str());
            } else {
                // Appends the set of issue keys, if project clause exists
                let s = format!("{jql} and issuekey in ({keys_spec})");
                jql = s;
            }
        }

        let jql_encoded = urlencoding::encode(&jql);

        let jira_issues = self.fetch_jql_result(jql_encoded.as_ref()).await?;
        Ok(jira_issues)
    }

    /// Fetches paginated JQL results from the Jira server
    #[allow(clippy::cast_possible_wrap)]
    #[allow(clippy::cast_sign_loss)]
    async fn fetch_jql_result(&self, jql_encoded: &str) -> Result<Vec<Issue>> {
        let mut resource = Self::compose_resource_for_next_jql_page(jql_encoded, 0, 50);

        let mut jira_issues: Vec<Issue> = Vec::new();
        loop {
            let mut jira_issues_page = self.get::<IssuesPage>(&resource).await?;
            let last_page = jira_issues_page.issues.is_empty()
                || jira_issues_page.issues.len() < jira_issues_page.max_results as usize;
            if !last_page {
                resource = Self::compose_resource_for_next_jql_page(
                    jql_encoded,
                    jira_issues_page.start_at
                        + i32::try_from(jira_issues_page.issues.len()).unwrap(),
                    jira_issues_page.max_results,
                );
            }
            jira_issues.append(&mut jira_issues_page.issues);
            if last_page {
                break;
            }
        }
        Ok(jira_issues)
    }

    /// Composes the resource string for the next page of JQL results
    fn compose_resource_for_next_jql_page(
        jql_encoded: &str,
        start_at: i32,
        max_results: i32,
    ) -> String {
        let resource = format!(
            "/search?jql={}&startAt={}&maxResults={}&fields={}",
            jql_encoded, start_at, max_results, "id,key,summary,components,description"
        );
        resource
    }

    ///
    /// Retrieves all public Jira projects based on provided project keys,
    /// filtering out the private ones.
    ///
    /// Currently only used in examples
    ///
    /// This function filters out private projects automatically and handles paginated results
    /// from the Jira API, ensuring all public projects are retrieved.
    ///
    /// # Arguments
    ///
    /// * `project_keys` - List of project keys to fetch Jira projects for.
    ///
    /// # Returns
    ///
    /// A `Result` containing a `Vec` of `Project` if successful, or an error otherwise.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// * Network requests fail.
    /// * Parsing the response fails.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let jira_client = JiraClient::new("https://your-jira-instance.com", "username", "token");
    /// let project_keys = vec!["PRJ1".to_string(), "PRJ2".to_string()];
    /// let projects = jira_client.get_projects(project_keys).await?;
    /// for project in projects {
    ///     println!("Jira Project: {}", project.name);
    /// }
    /// ```
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

    /// Retrieves all Jira issues for a given project.
    ///
    /// This function handles paginated results from the Jira API to fetch all issues
    /// associated with a specific project. It ensures that all issues are collected by
    /// iterating over all available pages while avoiding any potential data loss.
    ///
    /// # Arguments
    ///
    /// * `project_key` - The key of the Jira project for which issues are being retrieved.
    ///
    /// # Returns
    ///
    /// A `Result` containing a `Vec` of `Issue` if successful, or an error otherwise.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// * Network requests fail.
    /// * Parsing the response fails.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let jira_client = JiraClient::new("https://your-jira-instance.com", "username", "token");
    /// let project_key = "PRJ1".to_string();
    /// let issues = jira_client.get_issues_for_project(project_key).await?;
    /// for issue in issues {
    ///     println!("Jira Issue: {}", issue.summary);
    /// }
    /// ```
    #[allow(
        clippy::cast_sign_loss,
        clippy::cast_possible_truncation,
        clippy::cast_possible_wrap
    )]
    pub async fn get_issues_for_project(&self, project_key: String) -> Result<Vec<Issue>> {
        let mut resource = Self::compose_resource_and_params(&project_key, 0, 1024);

        let mut issues = Vec::<Issue>::new();
        loop {
            let mut issue_page = self.get::<IssuesPage>(&resource).await?;

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
            let mut worklog_page = self.get::<WorklogsPage>(&resource_name).await?;
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

        let resource = format!(
            "/search?jql={}&startAt={}&maxResults={}&fields={}",
            jql_encoded, start_at, max_results, "summary"
        );
        resource
    }

    #[allow(dead_code)]
    #[allow(clippy::missing_errors_doc)]
    async fn augment_projects_with_their_issues(
        self,
        projects: Vec<Project>,
    ) -> Result<Vec<Project>> {
        let mut futures_stream = futures::stream::iter(projects)
            .map(|mut project| {
                let me = self.clone();
                tokio::spawn(async move {
                    let Ok(issues) = me.get_issues_for_project(project.key.clone()).await else {
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

    /// Retrieves issues and their associated worklogs for the specified projects.
    ///
    /// # Arguments
    /// * `projects` - A vector of `Project` instances from which to retrieve issues and worklogs.
    /// * `issues_filter` - A vector of issue keys to filter issues by. If empty, all issues are included.
    /// * `started_after` - A `NaiveDateTime` instance to filter worklog entries that started after this date.
    ///
    /// # Returns
    /// A `Result` containing a vector of updated `Project` instances with their issues and associated worklogs,
    /// or an error if any operation fails.
    ///
    /// # Errors
    /// This function can return an error if:
    /// * Retrieving issues for a specific project fails.
    /// * Retrieving worklogs for an issue fails.
    /// * Internal futures processing encounters a failure.
    ///
    /// # Example
    /// ```rust,ignore
    /// use chrono::NaiveDate;
    ///
    /// let projects = vec![]; // Assume projects are populated.
    /// let issues_filter = vec!["ISSUE-1".to_string(), "ISSUE-2".to_string()];
    /// let started_after = NaiveDate::from_ymd(2023, 10, 01).and_hms(0, 0, 0);
    ///
    /// let result = instance.get_issues_and_worklogs(projects, issues_filter, started_after).await;
    /// match result {
    ///     Ok(updated_projects) => println!("Retrieved {} projects", updated_projects.len()),
    ///     Err(e) => println!("Error: {}", e),
    /// }
    /// ```
    #[allow(dead_code)]
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
                    let issues = me.get_issues_for_project(project.key.clone()).await?;
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
                        let mut worklogs = match me.get_worklogs_for(key, started_after).await {
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

    /// Inserts a worklog for a specific issue in Jira.
    ///
    /// This function is used to log work time for a Jira issue. It formats the `started` time
    /// based on the Jira-supported date-time format and then sends the worklog data to the Jira server.
    ///
    /// # Parameters
    /// - `issue_id`: The ID of the Jira issue for which the worklog will be logged.
    /// - `started`: The starting date and time of the worklog, formatted as `DateTime<Local>`.
    /// - `time_spent_seconds`: The duration of the worklog in seconds.
    /// - `comment`: A description or comment about the work performed.
    ///
    /// # Returns
    /// - `Ok(Worklog)` if the operation succeeds, containing the created worklog entry.
    /// - An error of type `Result<Worklog, E>` if the operation fails (e.g., network error or invalid input).
    ///
    /// # Notes
    /// - The `started` time format includes timezone information and is based on the user's local time.
    /// - Ensure that the provided `issue_id` corresponds to an existing Jira issue and that the user
    ///   has the appropriate permissions to log time.
    ///
    /// # Errors
    /// This function may return:
    /// - An error related to network communication if the server cannot be reached.
    /// - Validation errors if the input data or formatting does not meet Jira's requirements.
    ///
    /// # Example
    /// ```rust,ignore
    /// use chrono::Local;
    ///
    /// let started = Local::now();
    /// let time_spent_seconds = 3600; // 1 hour
    /// let comment = "Worked on improving project documentation.";
    ///
    /// match instance.insert_worklog("ISSUE-123", started, time_spent_seconds, comment).await {
    ///     Ok(worklog) => println!("Successfully inserted worklog: {:?}", worklog),
    ///     Err(e) => eprintln!("Error inserting worklog: {:?}", e),
    /// }
    /// ```
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

    /// Creates a new issue in Jira.
    ///
    /// This function creates an issue for a specified Jira project key with provided
    /// details such as summary and an optional description. The issue is created with
    /// the task type "Task".
    ///
    /// # Parameters
    /// - `jira_project_key`: The key of the Jira project where the new issue will be created.
    /// - `summary`: A brief summary or title for the new issue.
    /// - `description`: An optional detailed description of the issue.
    ///
    /// # Returns
    /// - `Ok(JiraNewIssueResponse)` if the issue is successfully created, containing details about the created issue.
    /// - Returns an appropriate error if the creation fails due to network issues, invalid project key,
    ///   or lack of permissions.
    ///
    /// # Errors
    /// This function may return:
    /// - `JiraError::NetworkError` if a network communication issue occurs while interacting with the Jira API.
    /// - `JiraError::InvalidResponse` if the server provides an invalid or unexpected response.
    ///
    /// # Example
    /// ```rust,ignore
    /// let jira_project_key = JiraProjectKey { key: "PROJ".to_string() };
    /// let summary = "Implement new feature";
    /// let description = Some("This will implement a new major feature for the project.".to_string());
    ///
    /// match instance.create_issue(&jira_project_key, &summary, description).await {
    ///     Ok(response) => println!("Issue created successfully: {:?}", response),
    ///     Err(e) => eprintln!("Failed to create issue: {:?}", e),
    /// }
    /// ```
    pub async fn create_issue(
        &self,
        jira_project_key: &JiraProjectKey,
        summary: &str,
        description: Option<String>,
    ) -> Result<JiraNewIssueResponse> {
        let new_issue = JiraNewIssue {
            fields: JiraIssueFields {
                project: JiraProjectKey {
                    key: jira_project_key.key,
                },
                issuetype: JiraIssueType {
                    name: "Task".to_string(),
                },
                summary: summary.to_string(),
                description,
            },
        };

        let url = "/issue";

        let result = self
            .post::<JiraNewIssueResponse, JiraNewIssue>(url, new_issue)
            .await?;
        debug!("Created issue {:?}", result);
        Ok(result)
    }

    /// Deletes an existing worklog associated with a specific issue.
    ///
    /// This function interacts with the Jira server to delete a worklog entry
    /// by its corresponding issue ID and worklog ID.
    ///
    /// # Parameters
    /// - `issue_id`: The ID of the issue to which the worklog belongs.
    /// - `worklog_id`: The ID of the worklog to be deleted.
    ///
    /// # Returns
    /// - Returns `Ok(())` on successful deletion of the worklog.
    /// - Returns an appropriate error if the operation fails, such as network or permission-related issues.
    ///
    /// # Errors
    /// This function may return:
    /// - `WorklogError::NetworkError` if there's an issue connecting to the server.
    /// - `WorklogError::JiraResponse` if Jira responds with an error (e.g., invalid worklog ID or insufficient permissions).
    pub async fn delete_worklog(&self, issue_id: String, worklog_id: String) -> Result<()> {
        let url = format!("/issue/{}/worklog/{}", &issue_id, &worklog_id);
        let _ = self.delete::<Option<Worklog>>(&url).await?;
        Ok(())
    }

    /// Deletes an existing Jira issue.
    ///
    /// This function interacts with the Jira server to delete a specified issue
    /// by its unique key. Once deleted, the issue will no longer be accessible
    /// in the Jira system.
    ///
    /// # Parameters
    /// - `jira_key`: A reference to the `JiraKey` representing the unique key of the issue to delete.
    ///
    /// # Returns
    /// - Returns `Ok(())` on successful deletion of the issue.
    /// - Returns an appropriate error if the operation fails, such as network issues or
    ///   authentication problems.
    ///
    /// # Errors
    /// This function may return:
    /// - `WorklogError::NetworkError` if there's a problem establishing a connection.
    /// - `WorklogError::JiraResponse` if Jira responds with an error (e.g., issue not found or insufficient permissions).
    pub async fn delete_issue(&self, jira_key: &JiraKey) -> Result<()> {
        let url = format!("/issue/{}", jira_key.value);
        self.delete::<Option<JiraKey>>(&url).await?;
        Ok(())
    }

    /// Fetches information about the currently authenticated user.
    ///
    /// This function sends a request to the Jira server to retrieve details about
    /// the user connected to the provided credentials. The retrieved user information
    /// includes account ID, email address, display name, and so on.
    ///
    /// # Returns
    /// - Returns a `Result` containing the `User` object on success.
    /// - If the operation fails, it returns an appropriate error, such as a network error or
    ///   authentication failure.
    ///
    /// # Errors
    /// This function may return:
    /// - `WorklogError::NetworkError` if there's a problem connecting to the Jira server.
    /// - `WorklogError::JiraResponse` if Jira responds with an error, such as invalid credentials
    ///   or insufficient permissions.
    pub async fn get_current_user(&self) -> Result<User> {
        self.get::<User>("/myself").await
    }

    /// Retrieves the available time tracking options configured in Jira.
    ///
    /// This function queries the Jira server for global time tracking settings.
    /// The result includes information about the time tracking configuration
    /// such as the estimates field used, time tracking provider, and other
    /// related details.
    ///
    /// # Returns
    /// - Returns a `Result` containing the `TimeTrackingConfiguration` object on success.
    /// - Returns an appropriate error if the operation fails, such as network issues or
    ///   authentication problems.
    ///
    /// # Errors
    /// This function may return:
    /// - `WorklogError::NetworkError` if there's an issue connecting to the server.
    /// - `WorklogError::JiraResponse` if Jira responds with an error, such as insufficient permissions.
    pub async fn get_time_tracking_options(&self) -> Result<TimeTrackingConfiguration> {
        let global_settings = self.get::<GlobalSettings>("/configuration").await?;
        Ok(global_settings.timeTrackingConfiguration)
    }

    /// Retrieves a specific worklog for a given issue.
    ///
    /// This function fetches a worklog corresponding to the provided issue ID
    /// and worklog ID. It communicates with the Jira API to retrieve the details
    /// of the worklog entry.
    ///
    /// # Parameters
    /// - `issue_id`: A string slice representing the ID of the issue to which the worklog belongs.
    /// - `worklog_id`: A string slice representing the unique ID of the worklog to retrieve.
    ///
    /// # Returns
    /// - Returns a `Result` containing the `Worklog` object on successful retrieval.
    /// - If the operation fails, it returns an appropriate error, such as network issues or
    ///   Jira API-related errors.
    ///
    /// # Errors
    /// This function may return:
    /// - `WorklogError::NetworkError` if there's an issue connecting to the Jira server.
    /// - `WorklogError::JiraResponse` if Jira responds with an error, such as an invalid worklog ID
    ///   or insufficient permissions.
    pub async fn get_worklog(&self, issue_id: &str, worklog_id: &str) -> Result<Worklog> {
        let resource = format!("/issue/{issue_id}/worklog/{worklog_id}");
        self.get::<Worklog>(&resource).await
    }

    /// Retrieves all worklogs for the currently authenticated user associated with a specific issue.
    ///
    /// This function fetches worklogs for a given Jira issue key and filters the results
    /// to include only those created by the currently authenticated user. Optionally,
    /// it allows filtering worklogs started after a specified date.
    ///
    /// # Parameters
    /// - `issue_key`: A string slice representing the key of the Jira issue.
    /// - `started_after`: An optional `DateTime<Local>` object representing the timestamp after which
    ///   worklogs should be included. If omitted, defaults to approximately one month prior to the current date.
    ///
    /// # Returns
    /// - Returns a `Result` containing a vector of `Worklog` objects authored by the currently authenticated user on success.
    /// - If the operation fails, it returns an appropriate error, such as network issues or Jira API-related errors.
    ///
    /// # Errors
    /// This function may return:
    /// - `WorklogError::NetworkError` if there's an issue connecting to the Jira server.
    /// - `WorklogError::JiraResponse` if the Jira API responds with an error, such as insufficient permissions or issue not found.
    ///
    /// # Panics
    /// This function will panic if `issue_key` is an empty string.
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
        let result = self
            .get_worklogs_for(issue_key.to_string(), naive_date_time)
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
    async fn fetch_myself_success() -> Result<()> {
        let mut server = Server::new_async().await;
        let url = server.url();
        let _m = server
            .mock("GET", "/rest/api/latest/myself")
            .with_status(200)
            .with_body(
                r#"{
                "self": "foo",
                "accountId": "foo",
                "emailAddress": "foo@bar.com",
                "displayName": "foo",
                "timeZone": "local"
            }"#,
            )
            .create_async()
            .await;

        let client = Jira::new(
            url,
            Credentials::Basic("foo@bar.com".to_string(), String::new()),
        )?;
        let user = client.get_current_user().await?;

        assert_eq!(user.email_address, "foo@bar.com");
        Ok(())
    }

    #[tokio::test]
    async fn fetch_myself_unauth() -> Result<()> {
        let mut server = Server::new_async().await;
        let url = server.url();
        let _m = server
            .mock("GET", "/rest/api/latest/myself")
            .with_status(403)
            .with_body(
                r#"{
                "errorMessages": ["foo"],
                "errors": {}
            }"#,
            )
            .create_async()
            .await;

        let client = Jira::new(
            url,
            Credentials::Basic("foo@bar.com".to_string(), String::new()),
        )?;
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
