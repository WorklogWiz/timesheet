//!
//! `jira_lib` is a collection of useful functions when interacting with
//! Jira using the official REST interface.
//!
//! Many of the types_old have been declared specifically for the purpose of work log management,
//! and are hence not generic.
use std::{
    collections::BTreeMap,
    error::Error,
    fmt::{self, Formatter},
};

use chrono::{DateTime, Days, Local, NaiveDateTime, TimeZone};
use futures::{stream, StreamExt};
use log::{debug, warn};
use models::{
    project::{JiraProjectsPage, Project},
    user::User,
    worklog::{Insert, Worklog, WorklogsPage},
};
use reqwest::{
    header::{ACCEPT, CONTENT_TYPE},
    Client, Method, RequestBuilder, StatusCode,
};

use crate::models::core::IssueKey;
use crate::models::issue::{
    ComponentId, IssueSummary, IssueType, IssuesResponse, NewIssue, NewIssueFields,
    NewIssueResponse,
};
use crate::models::project::{Component, JiraProjectKey};
use crate::models::setting::{GlobalSettings, TimeTrackingConfiguration};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use url::{ParseError, Url};

pub mod models;

type Result<T> = std::result::Result<T, JiraError>;

const MAX_RESULTS: i32 = 100; // Value of Jira `maxResults` variable when fetching data

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
    UriTooLong(String),
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
            UriTooLong(uri) => write!(f, "URI too long: {uri} "),
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
            StatusCode::URI_TOO_LONG => Err(JiraError::UriTooLong(url.to_string())),
            client_err if client_err.is_client_error() => {
                eprintln!("ERROR: http GET returned {status} for {url}, reason:{body}");
                Err(JiraError::Fault {
                    code: status,
                    errors: serde_json::from_str::<Errors>(body)?,
                })
            }
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

    /// Fetches issues from Jira using a specified JQL query and response fields.
    ///
    /// This function sends a JQL query to the Jira server to retrieve issues that
    /// match the specified criteria. This supports pagination and will continue
    /// fetching until all issues are retrieved.
    ///
    ///
    /// # Parameters
    /// - `jql`: A reference to a string containing the JQL query.
    /// - `fields`: A vector of field names to include in the response.
    ///
    /// # Returns
    /// - Returns a `Result` containing a vector of issues of type `T` on success.
    /// - Returns an appropriate error if the operation fails, such as network issues
    ///   or authentication problems.
    ///
    /// # Errors
    /// This function may return:
    /// - `WorklogError::NetworkError` if there's an issue connecting to the server.
    /// - `WorklogError::JiraResponse` if Jira responds with an error, such as invalid query syntax
    ///   or permissions issues.
    pub async fn fetch_with_jql<T>(&self, jql: &str, fields: Vec<&str>) -> Result<Vec<T>>
    where
        T: DeserializeOwned,
    {
        let jql_encoded = urlencoding::encode(jql);
        let mut results: Vec<T> = Vec::new();

        let mut next_page_token = None;
        loop {
            let resource = if let Some(token) = next_page_token {
                format!(
                    "/search/jql?jql={}&fields={}&maxResults={}&nextPageToken={}",
                    jql_encoded,
                    fields.join(","),
                    MAX_RESULTS,
                    token
                )
            } else {
                format!(
                    "/search/jql?jql={}&fields={}&maxResults={}",
                    jql_encoded,
                    fields.join(","),
                    MAX_RESULTS
                )
            };
            debug!("http get '{:?}'", resource);
            let response: IssuesResponse<T> = self.get(&resource).await?;
            results.extend(response.issues);

            if let Some(token) = response.next_page_token {
                next_page_token = Some(token);
            } else {
                break;
            }
        }
        Ok(results)
    }

    /// Searches for Jira issues where `worklogAuthor` IS NOT EMPTY
    /// based on provided projects and/or issue keys.
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
    pub async fn get_issue_summaries(
        &self,
        project_filter: &Vec<&str>,
        issue_key_filter: &[IssueKey],
        all_users: bool,
    ) -> Result<Vec<IssueSummary>> {
        if project_filter.is_empty() && issue_key_filter.is_empty() {
            warn!("No projects or issue keys provided");
            return Ok(vec![]);
        }

        let mut jql = String::new();

        if !project_filter.is_empty() {
            jql = format!("project in ({})", project_filter.join(","));
        }
        if !issue_key_filter.is_empty() {
            // creates comma-separated list of issue the keys
            let keys_spec = issue_key_filter
                .iter()
                .map(std::string::ToString::to_string)
                .collect::<Vec<_>>()
                .join(",");

            if jql.is_empty() {
                // No Project clause, so only add the issue keys
                jql.push_str(format!("issueKey in ({keys_spec})").as_str());
            } else {
                // Appends the set of issue keys, after project filter
                let s = format!("{jql} and issueKey in ({keys_spec})");
                jql = s;
            }
        }
        if all_users {
            jql.push_str(" AND worklogAuthor is not EMPTY ");
        } else {
            jql.push_str(" AND worklogAuthor=currentUser() ");
        }
        debug!("search_issues() :- Composed this JQL: {jql}");

        self.fetch_with_jql(
            &jql,
            vec!["id", "key", "summary", "components"], // TODO: Add "component" to list of fields to retrieve
        )
        .await
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

        // TODO: replace project search with pagination logic startAt, isLast, etc.
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

    ///
    /// Retrieves all components for a specific Jira project.
    ///
    /// This function queries the Jira API to fetch all components associated with the
    /// provided project key. Components in Jira are used to organize and classify issues
    /// within a project.
    ///
    /// # Arguments
    ///
    /// * `project_key` - A reference to a string slice that specifies the key of the Jira project.
    ///
    /// # Returns
    ///
    /// A `Result` containing a `Vec` of `Component` if successful, or an error if the request fails.
    ///
    /// # Errors
    ///
    /// This function returns an error if:
    /// - The network request to the Jira API fails.
    /// - Parsing of the API response fails.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let jira_client = JiraClient::new("https://your-jira-instance.com", "username", "token");
    /// let project_key = "PRJ1";
    /// let components = jira_client.get_components(project_key).await?;
    /// for component in components {
    ///     println!("Component: {}", component.name);
    /// }
    /// ```
    pub async fn get_components(&self, project_key: &str) -> Result<Vec<Component>> {
        let url = format!("/project/{project_key}/components?componentSource=auto");
        let components = self.get::<Vec<Component>>(&url).await?;

        Ok(components)
    }

    ///
    /// Retrieves all work logs for a specific Jira issue, starting from a given time.
    ///
    /// This function fetches paginated work logs for a Jira issue by querying the Jira API.
    /// It continues retrieving work logs until no more pages are available.
    ///
    /// # Arguments
    ///
    /// * `issue_key` - The key of the Jira issue for which work logs are being retrieved.
    /// * `started_after` - A `NaiveDateTime` indicating the cutoff time for the work logs to retrieve.
    ///
    /// # Returns
    ///
    /// A `Result` containing a `Vec` of `Worklog` if successful, or an error otherwise.
    ///
    /// # Errors
    ///
    /// This function returns an error if:
    /// * Network requests to retrieve worklogs fail.
    /// * Parsing the Jira API responses fails.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let jira_client = JiraClient::new("https://your-jira-instance.com", "username", "token");
    /// let issue_key = "ISSUE-123".to_string();
    /// let started_after = NaiveDateTime::parse_from_str("2023-01-01 00:00:00", "%Y-%m-%d %H:%M:%S")?;
    /// let worklogs = jira_client
    ///     .get_worklogs_for_issue(issue_key, started_after)
    ///     .await?;
    /// for worklog in worklogs {
    ///     println!("Worklog author: {}, time spent: {}", worklog.author, worklog.time_spent);
    /// }
    /// ```
    /// # Panics
    ///
    /// This function will panic if:
    /// - The `issue_key` is empty, as it is required to specify an issue key.
    ///
    /// Ensure that the `issue_key` parameter is properly provided before calling this method.
    #[allow(
        clippy::cast_sign_loss,
        clippy::cast_possible_truncation,
        clippy::cast_possible_wrap
    )]
    pub async fn get_work_logs_for_issue(
        &self,
        issue_key: &IssueKey,
        started_after: NaiveDateTime,
    ) -> Result<Vec<Worklog>> {
        assert!(!issue_key.is_empty(), "Must specify an issue key");
        let mut resource_name =
            Self::compose_work_logs_url(issue_key.as_str(), 0, 5000, started_after);
        let mut worklogs: Vec<Worklog> = Vec::<Worklog>::new();

        debug!("Retrieving work logs for {}", issue_key);
        // Loops through the result pages until last page received
        loop {
            let mut worklog_page = self.get::<WorklogsPage>(&resource_name).await?;
            let is_last_page = worklog_page.worklogs.len() < worklog_page.max_results as usize;
            if !is_last_page {
                resource_name = Self::compose_work_logs_url(
                    issue_key.as_str(),
                    worklog_page.startAt + worklog_page.worklogs.len(),
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
    pub async fn get_work_log_by_isssue_and_id(
        &self,
        issue_id: &str,
        worklog_id: &str,
    ) -> Result<Worklog> {
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
    pub async fn get_work_logs_for_current_user(
        &self,
        issue_key: &str,
        started_after: Option<DateTime<Local>>,
    ) -> Result<Vec<Worklog>> {
        assert!(!issue_key.is_empty(), "Must specify an issue key");
        let date_time = started_after.unwrap_or_else(|| {
            // Defaults to a month (approx)
            Local::now().checked_sub_days(Days::new(30)).unwrap()
        });
        let naive_date_time = DateTime::from_timestamp_millis(date_time.timestamp_millis())
            .unwrap()
            .naive_local();
        let result = self
            .get_work_logs_for_issue(&IssueKey::new(issue_key), naive_date_time)
            .await?;
        debug!("Work logs retrieved, filtering them for current user ....");
        let current_user = self.get_current_user().await?;
        Ok(result
            .into_iter()
            .filter(|wl| wl.author.accountId == current_user.account_id)
            .collect())
    }

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

    fn compose_work_logs_url(
        issue_key: &str,
        start_at: usize,
        max_results: usize,
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
        components: Vec<ComponentId>,
    ) -> Result<NewIssueResponse> {
        let new_issue = NewIssue {
            fields: NewIssueFields {
                project: JiraProjectKey {
                    key: jira_project_key.key,
                },
                issuetype: IssueType {
                    name: "Task".to_string(),
                },
                summary: summary.to_string(),
                description,
                components,
            },
        };

        let url = "/issue";

        let result = self
            .post::<NewIssueResponse, NewIssue>(url, new_issue)
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
    pub async fn delete_issue(&self, jira_key: &IssueKey) -> Result<()> {
        let url = format!("/issue/{}", jira_key.value);
        self.delete::<Option<IssueKey>>(&url).await?;
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

    ///
    /// Fetches work logs for a list of issues in chunks, starting after the specified naive date-time.
    ///
    /// This function retrieves worklogs asynchronously for a collection of issues. It requests
    /// worklog data for each issue key provided in the `issue_keys` parameter and starts
    /// fetching worklogs chronologically after the given `start_after_naive_date_time`.
    ///
    /// The function leverages asynchronous buffering to request data concurrently for up to 10
    /// issues at a time, merging results into a single collection.
    ///
    /// # Parameters
    /// - `issue_keys`: A reference to a vector of `IssueKey` objects representing the Jira issues
    ///   for which worklogs should be retrieved.
    /// - `start_after_naive_date_time`: A `NaiveDateTime` instance representing the cutoff point
    ///   for retrieving worklogs. Only worklogs created or updated after this date-time will be fetched.
    ///
    /// # Returns
    /// - Returns a `Result` containing a `Vec<Worklog>` on success.
    /// - Returns an appropriate error if any of the requests fail.
    ///
    /// # Errors
    /// This function may return:
    /// - `WorklogError::NetworkError` if there's a problem with the connection.
    /// - `WorklogError::JiraResponse` if an error occurs in any of the Jira server responses.
    pub async fn chunked_work_logs(
        &self,
        issue_keys: &Vec<IssueKey>,
        start_after_naive_date_time: NaiveDateTime,
    ) -> Result<Vec<Worklog>> {
        let futures = stream::iter(issue_keys)
            .map(|key| self.get_work_logs_for_issue(key, start_after_naive_date_time))
            .buffer_unordered(10);

        let issue_worklogs: Vec<_> = futures
            .filter_map(|result| async {
                match result {
                    Ok(worklogs) => Some(worklogs),
                    Err(_) => None,
                }
            })
            .concat()
            .await;

        Ok(issue_worklogs)
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
