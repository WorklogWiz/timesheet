//!
//! `jira_lib` is a collection of useful functions when interacting with
//! Jira using the official REST interface.
//!
//! Many of the types have been declared specifically for the purpose of work log management,
//! and are hence not generic.
extern crate core;

use std::cmp::Ordering;
use std::error::Error;
use std::fmt;
use std::fmt::Formatter;
use std::process::exit;
use std::time::Instant;

use base64::Engine;
use base64::prelude::*;
use chrono::{DateTime, Days, Local, Months, NaiveDateTime, NaiveTime, TimeZone, Utc};
use futures::StreamExt;
use log::{debug, info};
use reqwest::{Client, StatusCode};
use reqwest::header::HeaderMap;
use reqwest::header::HeaderValue;
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde::Serialize;

use common::config;
use config::ApplicationConfig;

/// Holds the ULR of the Jira API to use
pub const JIRA_URL: &str = "https://autostore.atlassian.net/rest/api/latest";
const FUTURE_BUFFER_SIZE: usize = 20;
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

/// Represents the global Jira settings
#[derive(Debug, Serialize, Deserialize)]
#[allow(non_snake_case)]
#[allow(clippy::struct_excessive_bools)]
pub struct GlobalSettings {
    votingEnabled: bool,
    watchingEnabled: bool,
    unassignedIssuesAllowed: bool,
    subTasksEnabled: bool,
    issueLinkingEnabled: bool,
    timeTrackingEnabled: bool,
    attachmentsEnabled: bool,
    timeTrackingConfiguration: TimeTrackingConfiguration,
}

/// Represents the time tracking configuration settings retrieved from Jira
#[derive(Debug, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct TimeTrackingConfiguration {
    /// Holds the number of work hours per day, typically 7.5 in Norway
    pub workingHoursPerDay: f32,
    /// Number of work days per week, typically 5.0
    pub workingDaysPerWeek: f32,
    /// What time format is used
    pub timeFormat: String,
    /// What is the default unit
    pub defaultUnit: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct WorklogsPage {
    pub startAt: i32,
    #[serde(alias = "maxResults")]
    pub max_results: i32,
    pub total: i32,
    pub worklogs: Vec<Worklog>,
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, PartialOrd)]
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
    pub issueId: String,    // Numeric FK to issue
    pub comment: Option<String>,
}

/// Represents the author (user) of a worklog item
#[derive(Debug, Deserialize, Serialize, PartialOrd, PartialEq, Eq, Hash, Clone)]
#[allow(non_snake_case)]
pub struct Author {
    pub accountId: String,
    pub emailAddress: Option<String>,
    pub displayName: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[allow(non_snake_case)]
pub struct JiraProjectsPage {
    pub nextPage: Option<String>,
    pub startAt: i32,
    pub maxResults: i32,
    pub total: Option<i32>,
    pub isLast: Option<bool>,
    pub values: Vec<JiraProject>,
}

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct JiraProject {
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
    pub issues: Vec<JiraIssue>,
}

/// Represents a page of Jira issues retrieved from Jira
#[derive(Debug, Deserialize, Serialize)]
#[allow(non_snake_case)]
pub struct JiraIssuesPage {
    #[serde(alias = "startAt")]
    pub start_at: i32,
    #[serde(alias = "maxResults")]
    pub max_results: i32,
    pub total: Option<i32>,
    pub isLast: Option<bool>,
    pub issues: Vec<JiraIssue>,
}
/// Represents a Jira issue key like for instance `TIME-148`
#[derive(Debug, Deserialize, Serialize, Default, Eq, PartialEq, Clone)]
pub struct JiraKey(pub String);

impl JiraKey {
    #[must_use]
    pub fn value(&self) -> &str {
        &self.0
    }
}
impl fmt::Display for JiraKey {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl Ord for JiraKey {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.to_uppercase().cmp(&other.0.to_uppercase())
    }
}
impl PartialOrd for JiraKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Represents a jira issue
#[derive(Debug, Deserialize, Serialize, Default)]
pub struct JiraIssue {
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

/// Holds the Jira custom field `customfield_10904`
#[derive(Debug, Deserialize, Serialize, Default)]
#[allow(non_snake_case)]
pub struct JiraFields {
    pub summary: String,
    #[serde(alias = "customfield_10904")]
    pub asset: Option<JiraAsset>,
}

#[derive(Debug, Deserialize, Serialize, Default)]
#[allow(non_snake_case)]
pub struct JiraAsset {
    #[serde(alias = "self")]
    pub url: String,
    pub id: String,
    pub value: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct WorklogInsert {
    pub comment: String,
    pub started: String,
    pub timeSpentSeconds: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JiraUser {
    #[serde(alias = "self")]
    pub self_url: String,
    #[serde(alias = "accountId")]
    pub account_id: String,
    #[serde(alias = "emailAddress")]
    pub email_address: String,
    #[serde(alias = "displayName")]
    pub display_name: String,
    #[serde(alias = "timeZone")]
    pub time_zone: String,
}

#[derive(Debug)]
pub enum JiraError {
    RequiredParameter(String),
    DeleteFailed(reqwest::StatusCode),
    WorklogNotFound(String, String),
    ReqwestError(reqwest::Error),
}

impl fmt::Display for JiraError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            JiraError::RequiredParameter(param_name) => {
                write!(f, "Parameter '{param_name}' must contain a a value")
            }
            JiraError::DeleteFailed(sc) => {
                write!(f, "Failed to delete: {sc}")
            }
            JiraError::WorklogNotFound(issue, worklog_id) => {
                write!(
                    f,
                    "Worklog entry with issue_key: {issue} and worklog_id: {worklog_id} not found")
            }
            JiraError::ReqwestError(e) => {
                write!(
                    f,
                    "Internal error in reqwest library: {}",
                    e.to_string().as_str()
                )
            }
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

/// Convenience method to create a `JiraClient` instance. It will load parameters
/// from the .toml file on disk and set up everything for you.
#[must_use]
#[allow(clippy::missing_panics_doc)]
pub fn create_jira_client() -> JiraClient {
    // Creates HTTP client with all the required credentials
    let config = config::load().unwrap();
    JiraClient::from(&config).unwrap()
}

pub struct JiraClient {
    pub jira_url: String,
    pub user_name: String,
    pub http_client: Client,
}

impl JiraClient {
    #[allow(clippy::missing_errors_doc)]
    pub fn new(jira_url: &str, user_name: &str, token: &str) -> Result<JiraClient, JiraError> {
        if jira_url.is_empty() {
            return Err(JiraError::RequiredParameter("jira_url".to_string()));
        }
        if user_name.is_empty() {
            return Err(JiraError::RequiredParameter("user_name".to_string()));
        }
        if token.is_empty() {
            return Err(JiraError::RequiredParameter("token".to_string()));
        }

        let http_client = match Self::create_http_client(user_name, token) {
            Ok(cl) => cl,
            Err(e) => return Err(JiraError::ReqwestError(e)),
        };

        Ok(JiraClient {
            jira_url: jira_url.to_string(),
            user_name: user_name.to_string(),
            http_client,
        })
    }

    #[allow(clippy::missing_errors_doc)]
    pub fn from(cfg: &ApplicationConfig) -> Result<JiraClient, JiraError> {
        JiraClient::new(&cfg.jira.jira_url,&cfg.jira.user, &cfg.jira.token)
    }

    fn create_http_client(user_name: &str, token: &str) -> Result<reqwest::Client, reqwest::Error> {
        debug!("create_http_client({},{})", user_name, token);
        reqwest::Client::builder()
            .default_headers(Self::create_default_headers(user_name, token))
            .build()
    }

    fn create_default_headers(user_name: &str, token: &str) -> HeaderMap {
        debug!("create_default_headers({},{}", user_name, token);

        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::AUTHORIZATION,
            HeaderValue::from_bytes(Self::create_auth_value(user_name, token).as_bytes()).unwrap(),
        );
        headers.insert(
            reqwest::header::CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
        );
        headers.insert(
            reqwest::header::ACCEPT,
            HeaderValue::from_static("application/json"),
        );
        headers
    }

    fn create_auth_value(user: &str, token: &str) -> String {
        debug!("create_auth_value({},{})", user, token);
        let mut s: String = String::from(user);
        s.push(':');
        s.push_str(token);
        let b64 = BASE64_STANDARD.encode(s.as_bytes());
        let authorisation = format!("Basic {b64}");
        debug!("Created this BASIC auth string: '{}'", authorisation);
        authorisation
    }

    #[allow(clippy::missing_errors_doc)]
    pub async fn get_time_tracking_options(
        &self,
    ) -> Result<TimeTrackingConfiguration, reqwest::StatusCode> {
        let resource = "/configuration";
        match Self::get_jira_resource::<GlobalSettings>(
            &self.http_client,
            resource,
            reqwest::StatusCode::OK,
        )
        .await
        {
            Ok(global_settings) => Ok(global_settings.timeTrackingConfiguration),
            Err(http_status) => Err(http_status),
        }
    }

    pub async fn get_projects_filtered(
        &self,
        filter_projects_opt: Option<Vec<String>>,
    ) -> Vec<JiraProject> {
        let filter = filter_projects_opt.unwrap_or_default();
        self.get_all_projects(filter).await
    }

    #[allow(clippy::missing_errors_doc)]
    pub async fn get_issue_by_id_or_key(&self, id: &str) -> Result<JiraIssue, reqwest::StatusCode> {
        let resource = format!("/issue/{id}");
        match Self::get_jira_resource::<JiraIssue>(
            &self.http_client,
            &resource,
            reqwest::StatusCode::OK,
        )
        .await
        {
            Ok(issue) => Ok(issue),
            Err(http_code) => Err(http_code),
        }
    }

    /// Retrieves all Jira projects, filtering out the private ones
    #[allow(clippy::missing_panics_doc)]
    pub async fn get_all_projects(&self, project_keys: Vec<String>) -> Vec<JiraProject> {
        let start_at = 0;

        // Retrieves first page of Jira projects
        let mut project_page = Self::get_jira_resource::<JiraProjectsPage>(
            &self.http_client,
            &Self::project_search_resource(start_at, project_keys),
            reqwest::StatusCode::OK,
        )
        .await
        .unwrap();

        let mut projects = Vec::<JiraProject>::new();
        if project_page.values.is_empty() {
            // No projects, just return empty vector
            return projects;
        }

        projects.append(
            &mut project_page
                .values
                .into_iter()
                .filter(|p| !p.is_private)
                .collect(),
        );

        // While there is a URL for the next page ...
        while let Some(url) = &project_page.nextPage {
            // Fetch next page of data
            project_page = Self::get_jira_data_from_url::<JiraProjectsPage>(
                &self.http_client,
                url.clone(),
                reqwest::StatusCode::OK,
            )
            .await
            .unwrap();
            // Filter out the private projects and append to our list of projects
            projects.append(
                &mut project_page
                    .values
                    .into_iter()
                    .filter(|p| !p.is_private)
                    .collect(),
            );
        }
        projects
    }

    pub async fn get_issues_for_single_project(&self, project_key: String) -> Vec<JiraIssue> {
        Self::static_get_issues_for_single_project(&self.http_client, project_key).await
    }

    // Calling an internal associated function from the public facing function, seems to be
    // the only way I can get away from the dreaded compiler error:
    //     | |__________________`self` escapes the associated function body here
    //     |                    argument requires that `'1` must outlive `'static`
    #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
    async fn static_get_issues_for_single_project(
        http_client: &Client,
        project_key: String,
    ) -> Vec<JiraIssue> {
        let mut resource = Self::compose_resource_and_params(&project_key, 0, 1024);

        let mut issues = Vec::<JiraIssue>::new();
        loop {
            let mut issue_page = match Self::get_jira_resource::<JiraIssuesPage>(
                http_client,
                &resource,
                reqwest::StatusCode::OK,
            )
            .await
            {
                Ok(p) => p,
                Err(e) => match e {
                    reqwest::StatusCode::NOT_FOUND => {
                        panic!("Project {project_key} not found");
                    }
                    other => {
                        panic!("Unexpected http status code {other} for {resource}")
                    }
                },
            };

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
        issues
    }

    #[allow(clippy::missing_errors_doc)]
    #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
    pub async fn get_worklogs_for(
        http_client: &Client,
        issue_key: String,
        started_after: NaiveDateTime,
    ) -> Result<Vec<Worklog>, StatusCode> {
        let mut resource_name =
            Self::compose_worklogs_url(issue_key.as_str(), 0, 5000, started_after);
        let mut worklogs: Vec<Worklog> = Vec::<Worklog>::new();

        debug!("Retrieving worklogs for {}", issue_key);
        loop {
            let mut worklog_page = match Self::get_jira_resource::<WorklogsPage>(
                http_client,
                &resource_name,
                reqwest::StatusCode::OK,
            )
            .await
            {
                Ok(wp) => wp,
                Err(http_code) => return Err(http_code),
            };
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

    async fn get_jira_resource<T: DeserializeOwned>(
        http_client: &Client,
        rest_resource: &str,
        code: StatusCode,
    ) -> Result<T, reqwest::StatusCode> {
        let url = format!("{JIRA_URL}{rest_resource}");

        Self::get_jira_data_from_url::<T>(http_client, url, code).await
    }

    async fn get_jira_data_from_url<T: DeserializeOwned>(
        http_client: &Client,
        url: String,
        expected_http_code: StatusCode,
    ) -> Result<T, reqwest::StatusCode> {
        let url_decoded = urlencoding::decode(&url).unwrap();
        debug!("Calling new get_jira_data_from_url({})", url_decoded);

        let start = Instant::now();
        let response = http_client.get(url.clone()).send().await.unwrap();
        let elapsed = start.elapsed();
        debug!("{} took {}ms", url, elapsed.as_millis());
        // Downloads the entire body of the response and convert from JSON to type safe struct
        if response.status() == expected_http_code {
            // Transforms JSON in body to type safe struct
            match response.json::<T>().await {
                Ok(wl) => {
                    debug!("Elapsed time for parsing {}", start.elapsed().as_millis());
                    return Ok(wl);
                } // Everything OK, return the Worklogs struct
                Err(err) => panic!("ERROR Obtaining response in JSON format: {err:?}"),
            }
        }
        Err(response.status())
    }

    // ---- END of static functions

    pub async fn get_issues_for_projects(&self, projects: Vec<JiraProject>) -> Vec<JiraProject> {
        let http_client = self.http_client.clone();
        let mut futures_stream = futures::stream::iter(projects)
            .map(|mut project| {
                let client = http_client.clone();
                tokio::spawn(async move {
                    let issues =
                        Self::static_get_issues_for_single_project(&client, project.key.clone())
                            .await;
                    let _old = std::mem::replace(&mut project.issues, issues);
                    project
                })
            })
            .buffer_unordered(FUTURE_BUFFER_SIZE);

        let mut result = Vec::<JiraProject>::new();
        while let Some(r) = futures_stream.next().await {
            match r {
                Ok(jp) => {
                    debug!("OK {}", jp.key);
                    result.push(jp);
                }
                Err(e) => eprintln!("Error: {e:?}"),
            }
        }
        result
    }

    // todo: clean up and make simpler!
    #[allow(clippy::missing_errors_doc, clippy::missing_panics_doc)]
    pub async fn get_issues_and_worklogs(
        &self,
        projects: Vec<JiraProject>,
        issues_filter: Vec<String>,
        started_after: NaiveDateTime,
    ) -> Result<Vec<JiraProject>, StatusCode> {
        let mut futures_stream = futures::stream::iter(projects)
            .map(|mut project| {
                let client = self.http_client.clone();
                debug!(
                    "Creating future for {}, filters={:?}",
                    &project.key, issues_filter
                );

                let filter = issues_filter.clone(); // Clones the vector to allow async move
                tokio::spawn(async move {
                    let issues =
                        Self::static_get_issues_for_single_project(&client, project.key.clone())
                            .await;
                    debug!(
                        "Extracted {} issues. Applying filter {:?}",
                        issues.len(),
                        filter
                    );
                    let issues: Vec<JiraIssue> = issues
                        .into_iter()
                        .filter(|issue| {
                            filter.is_empty() || !filter.is_empty() && filter.contains(&issue.key.0)
                        })
                        .collect();
                    debug!("Filtered {} issues for {}", issues.len(), &project.key);
                    let _old = std::mem::replace(&mut project.issues, issues);
                    for issue in &mut project.issues {
                        debug!("Retrieving worklogs for issue {}", &issue.key);
                        let key = issue.key.to_string();
                        let mut worklogs =
                            match Self::get_worklogs_for(&client, key, started_after).await {
                                Ok(result) => result,
                                Err(e) => return Err(e),
                            };
                        debug!(
                            "Issue {} has {} worklog entries",
                            issue.key.0,
                            worklogs.len()
                        );
                        issue.worklogs.append(&mut worklogs);
                    }
                    Ok(project)
                })
            })
            .buffer_unordered(FUTURE_BUFFER_SIZE);

        let mut result = Vec::<JiraProject>::new();
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

    #[allow(clippy::missing_errors_doc, clippy::missing_panics_doc)]
    pub async fn insert_worklog(
        &self,
        issue_id: &str,
        started: DateTime<Local>,
        time_spent_seconds: i32,
        comment: &str,
    ) -> Result<Worklog, StatusCode> {
        // This is how Jira needs it.
        // Note! The formatting in Jira is based on the time zone of the user. Remember to change it
        // if you fly across the ocean :-)
        // Move this into a function
        let start = started.format("%Y-%m-%dT%H:%M:%S.%3f%z");
        let worklog_entry = WorklogInsert {
            timeSpentSeconds: time_spent_seconds,
            comment: comment.to_string(),
            started: start.to_string(),
        };
        let json = serde_json::to_string(&worklog_entry).unwrap(); // Let Serde do the heavy lifting

        let url = format!("{}/issue/{}/worklog", self.jira_url, issue_id);
        debug!("Composed url for worklog insert: {}", url);

        Self::post_jira_data::<Worklog>(&self.http_client, url, json, StatusCode::CREATED).await
    }

    #[allow(clippy::missing_errors_doc, clippy::missing_panics_doc)]
    pub async fn delete_worklog(
        &self,
        issue_id: String,
        worklog_id: String,
    ) -> Result<(), JiraError> {
        let url = format!(
            "{}/issue/{}/worklog/{}",
            self.jira_url, &issue_id, &worklog_id
        );
        let response = self.http_client.delete(url).send().await.unwrap();
        match response.status() {
            reqwest::StatusCode::NO_CONTENT => Ok(()), // 204
            reqwest::StatusCode::NOT_FOUND => Err(JiraError::WorklogNotFound(issue_id, worklog_id)),
            other => Err(JiraError::DeleteFailed(other)),
        }
    }

    async fn post_jira_data<T: DeserializeOwned>(
        http_client: &Client,
        url: String,
        body: String,
        http_status_code: StatusCode,
    ) -> Result<T, StatusCode> {


        let response = http_client
            .post(url.clone())
            .body(body)
            .header("Content-Type", "application/json")
            .send()
            .await
            .unwrap();

        if response.status() == http_status_code {
            match response.json::<T>().await {
                Ok(worklog) => Ok(worklog),
                Err(e) => {
                    eprintln!("ERROR: posting to {url}");
                    eprintln!("ERROR: Unable to deserialize the json body {e:?}");
                    exit(4);
                }
            }
        } else {
            eprintln!("HTTP Post to {url}\n failed: {response:?}");
            Err(response.status())
        }
    }

    pub async fn get_current_user(&self) -> JiraUser {
        let resource = "/myself";
        match Self::get_jira_resource::<JiraUser>(
            &self.http_client,
            resource,
            reqwest::StatusCode::OK,
        )
        .await
        {
            Ok(ju) => ju,
            Err(StatusCode::UNAUTHORIZED) => {
                eprintln!("{resource} \nNot authorized (401) check your credentials");
                exit(4)
            }
            Err(e) => {
                eprintln!("ERROR: http status code {e} for {resource}");
                exit(4);
            }
        }
    }

    #[allow(clippy::missing_errors_doc)]
    pub async fn get_worklog(
        &self,
        issue_id: &str,
        worklog_id: &str,
    ) -> Result<Worklog, StatusCode> {
        let resource = format!("/issue/{issue_id}/worklog/{worklog_id}");
        Self::get_jira_resource::<Worklog>(&self.http_client, &resource, reqwest::StatusCode::OK)
            .await
    }

    #[allow(clippy::missing_errors_doc, clippy::missing_panics_doc)]
    pub async fn get_worklogs_for_current_user(
        &self,
        issue_key: &str,
        started_after: Option<DateTime<Local>>,
    ) -> Result<Vec<Worklog>, StatusCode> {
        assert!(!issue_key.is_empty(), "Must specify an issue key");
        let current_user = self.get_current_user().await;
        let date_time = started_after.unwrap_or_else(|| {
            // Defaults to a month (approx)
            Local::now().checked_sub_days(Days::new(30)).unwrap()
        });
        let naive_date_time = DateTime::from_timestamp_millis(date_time.timestamp_millis())
            .unwrap()
            .naive_local();
        let result =
            match Self::get_worklogs_for(&self.http_client, issue_key.to_string(), naive_date_time)
                .await
            {
                Ok(r) => r,
                Err(e) => match e {
                    StatusCode::NOT_FOUND => return Err(e),
                    other => panic!("Unexpected http code {other} for issue {issue_key}"),
                },
            };
        debug!("Work logs retrieved, filtering them for current user ....");
        Ok(result
            .into_iter()
            .filter(|wl| wl.author.accountId == current_user.account_id)
            .collect())
    }
}

#[must_use]
#[allow(clippy::missing_panics_doc)]
pub fn midnight_a_month_ago_in() -> NaiveDateTime {
    let today = chrono::offset::Local::now();
    let a_month_ago = today.checked_sub_months(Months::new(1)).unwrap();
    NaiveDateTime::new(
        a_month_ago.date_naive(),
        NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jira_key() {
        let k1 = JiraKey("TIME-40".to_string());
        let k2 = JiraKey("TIME-40".to_string());
        assert_eq!(&k1, &k2, "Seems JiraKey does not compare by value");
    }
}
