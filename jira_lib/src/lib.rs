extern crate core;


use std::time::Instant;

use chrono::{DateTime, Local, Months, NaiveDateTime, NaiveTime, Utc};
use futures::StreamExt;
use log::{debug, info};
use reqwest::{Client};
use reqwest::header::HeaderMap;
use reqwest::header::HeaderValue;
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde::Serialize;

pub const JIRA_URL: &str = "https://autostore.atlassian.net/rest/api/latest";

#[derive(Debug, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct TimeTrackingOptions {
    pub workingHoursPerDay: f32,
    pub workingDaysPerWeek: f32,
    pub timeFormat: String,
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

#[derive(Debug, Serialize, Deserialize)]
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
    pub issueId: String,        // Numeric FK to issue
}

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
    pub id: String,
    // numeric value
    pub key: String,
    // e.g. "TIME", "RGA", etc.
    pub name: String,
    #[serde(alias = "self")]
    pub url: String,
    #[serde(alias = "isPrivate")]
    pub is_private: bool,
    #[serde(skip)] // Added after Deserializing
    pub issues: Vec<JiraIssue>,
}

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


#[derive(Debug, Deserialize, Serialize, Default)]
pub struct JiraIssue {
    pub id: String,
    #[serde(alias = "self")]
    pub self_url: String,
    pub key: String,

    #[serde(skip)] // Added after deserializing
    pub worklogs: Vec<Worklog>,
    pub fields: JiraFields,
}

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


pub fn http_client() -> reqwest::Client {
    create_auth_value();

    match reqwest::Client::builder()
        .default_headers(create_default_headers())
        .build()
    {
        Ok(c) => c,
        Err(error) => panic!("Unable to create http client {:?}", error),
    }
}

fn create_default_headers() -> HeaderMap {
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        reqwest::header::AUTHORIZATION,
        HeaderValue::from_bytes(create_auth_value().as_bytes()).unwrap(),
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

// TODO: externalize the userid and the accompanying token
fn create_auth_value() -> String {
    let user = "steinar.cook@autostoresystem.com";
    let token = "vbFYbxdSeahS7KED9sK401E3";
    let mut s: String = String::from(user);
    s.push(':');
    s.push_str(token);
    let b64 = base64::encode(s.as_bytes());
    let authorisation = format!("Basic {}", b64);

    authorisation
}

pub async fn get_time_tracking_options(http_client: &Client) -> TimeTrackingOptions {
    let resource = "/configuration/timetracking/options";
    get_jira_resource::<TimeTrackingOptions>(http_client, resource).await
}

pub async fn get_projects_filtered(http_client: &Client, filter_projects_opt: Option<Vec<String>>) -> Vec<JiraProject> {
    let filter = filter_projects_opt.unwrap_or(vec![]);
    get_all_projects(http_client, filter).await
}

pub fn compose_project_urls(initial: i32, max_result: i32, total: i32) -> Vec<String> {
    let mut result = vec![];
    let mut start = initial;
    while start < total {
        result.push(compose_project_url(start, max_result));
        start += max_result;
    }
    result
}

#[test]
fn test_compose_urls() {
    assert_eq!(3, compose_project_urls(50, 50, 181).len());
}

pub fn compose_project_url(start_at: i32, max_results: i32) -> String {
    format!("{}/project/search?maxResults={}&startAt={}", JIRA_URL, max_results, start_at)
}

/// Retrieves all Jira projects, filtering out the private ones
pub async fn get_all_projects(http_client: &Client, project_keys: Vec<String>) -> Vec<JiraProject> {
    let start_at = 0;

    // Retrieves first page of Jira projects
    let mut project_page =
        get_jira_resource::<JiraProjectsPage>(http_client, &project_search_resource(start_at, project_keys))
            .await;

    let mut projects = Vec::<JiraProject>::new();
    if project_page.values.is_empty() {
        // No projects, just return empty vector
        return projects;
    }

    projects.append(&mut project_page.values.into_iter().filter(|p| !p.is_private).collect());

    // While there is a URL for the next page ...
    while let Some(url) = &project_page.nextPage {
        // Fetch next page of data
        project_page = get_jira_data_from_url::<JiraProjectsPage>(http_client, url.clone()).await;
        // Filter out the private projects and append to our list of projects
        projects.append(&mut project_page.values.into_iter().filter(|p| !p.is_private).collect());
    }
    projects
}

// TODO: Consider Trait with associated type as this logic is identical to get_worklogs_for()
pub async fn get_issues_for_single_project(http_client: &Client, project_key: String) -> Vec<JiraIssue> {
    let mut resource = compose_resource_and_params(project_key.to_owned(), 0, 1024);

    let mut issues = Vec::<JiraIssue>::new();
    loop {
        let mut issue_page = get_jira_resource::<JiraIssuesPage>(http_client, &resource).await;
        // issues.len() will be invalid once we move the contents of the issues into our result
        let is_last_page = issue_page.issues.len() < issue_page.max_results as usize;
        if !is_last_page {
            resource = compose_resource_and_params(
                project_key.to_owned(),
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

// TODO: Consider Trait with associated type as this logic is repeated twice
pub async fn get_worklogs_for(http_client: &Client, issue_key: String, started_after: NaiveDateTime) -> Vec<Worklog> {
    let mut resource_name = compose_worklogs_url(issue_key.as_str(), 0, 5000, started_after);
    let mut worklogs: Vec<Worklog> = Vec::<Worklog>::new();

    debug!("Retrieving worklogs for {}", issue_key);
    loop {
        let mut worklog_page = get_jira_resource::<WorklogsPage>(http_client, &resource_name).await;
        let is_last_page = worklog_page.worklogs.len() < worklog_page.max_results as usize;
        if !is_last_page {
            resource_name = compose_worklogs_url(
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
    worklogs
}


fn compose_worklogs_url(issue_key: &str, start_at: i32, max_results: i32, started_after: NaiveDateTime) -> String {
    format!(
        "/issue/{}/worklog?startAt={}&maxResults={}&startedAfter={}",
        issue_key, start_at, max_results, started_after.timestamp_millis()
    )
}

fn compose_resource_and_params(project_key: String, start_at: i32, max_results: i32) -> String {
    let jql = format!("project=\"{}\" and resolution=Unresolved", project_key);
    let jql_encoded = urlencoding::encode(&jql);

    // Custom field customfield_10904 is the project asset custom field
    let resource = format!(
        "/search?jql={}&startAt={}&maxResults={}&fields={}",
        jql_encoded, start_at, max_results, "summary,customfield_10904"
    );
    resource
}

pub async fn get_jira_resource<T: DeserializeOwned>(
    http_client: &Client,
    rest_resource: &str,
) -> T {
    let url = format!("{}{}", JIRA_URL, rest_resource);

    get_jira_data_from_url::<T>(http_client, url).await
}

pub async fn get_jira_data_from_url<T: DeserializeOwned>(http_client: &Client, url: String) -> T {
    let _url_decoded = urlencoding::decode(&url).unwrap();

    let start = Instant::now();
    let response = http_client.get(url.clone()).send().await.unwrap();
    let elapsed = start.elapsed();
    debug!("{} took {}ms", url, elapsed.as_millis());
    // Downloads the entire body of the response and convert from JSON to type safe struct
    let typed_result: T = match response.status() {
        reqwest::StatusCode::OK => {
            // Transforms JSON in body to type safe struct
            match response.json::<T>().await {
                Ok(wl) => {
                    debug!("Elapsed time for parsing {}", start.elapsed().as_millis());
                    wl
                } // Everything OK, return the Worklogs struct
                Err(err) => panic!("EROR Obtaining response in JSON format: {:?}", err),
            }
        }
        reqwest::StatusCode::UNAUTHORIZED => panic!("Not authorized, API key has probably changed"),
        reqwest::StatusCode::TOO_MANY_REQUESTS => {
            panic!("429 - Too many requests {:?}", response.headers())
        }

        other => {
            let decoded_url = urlencoding::decode(&url).unwrap();
            panic!(
                "Error code {:?} for {}\nencoded url={}",
                other, &decoded_url, &url
            );
        }
    };
    typed_result
}

fn project_search_resource(start_at: i32, project_keys: Vec<String>) -> String {
    // Seems 50 is the max value of maxResults
    let mut resource = format!("/project/search?maxResults=50&startAt={}", start_at);
    if !project_keys.is_empty() {
        for key in project_keys {
            resource.push_str("&keys=");
            resource.push_str(key.as_str());
        }
    }
    resource
}

pub async fn get_issues_for_projects(http_client: &Client, projects: Vec<JiraProject>) -> Vec<JiraProject> {
    let mut futures_stream = futures::stream::iter(projects)
        .map(|mut project| {
            let client = http_client.clone();
            tokio::spawn(async move {
                let issues = get_issues_for_single_project(&client, project.key.to_owned()).await;
                let _old = std::mem::replace(&mut project.issues, issues);
                project
            })
        }).buffer_unordered(FUTURE_BUFFER_SIZE);

    let mut result = Vec::<JiraProject>::new();
    while let Some(r) = futures_stream.next().await {
        match r {
            Ok(jp) => {
                debug!("OK {}", jp.key);
                result.push(jp);
            }
            Err(e) => eprintln!("Error: {:?}", e)
        }
    }
    result
}

const FUTURE_BUFFER_SIZE: usize = 20;

// todo: clean up and make simpler!
pub async fn get_issues_and_worklogs(http_client: &Client, projects: Vec<JiraProject>, issues_filter: Vec<String>, started_after: NaiveDateTime) -> Vec<JiraProject> {
    let mut futures_stream = futures::stream::iter(projects)
        .map(|mut project| {
            let client = http_client.clone();
            debug!("Creating future for {}, filters={:?}", &project.key, issues_filter);

            let filter = issues_filter.to_vec();    // Clones the vector to allow async move
            tokio::spawn(async move {
                let issues = get_issues_for_single_project(&client, project.key.to_owned()).await;
                debug!("Extracted {} issues. Applying filter {:?}", issues.len(), filter);
                let issues: Vec<JiraIssue> = issues.into_iter().filter(|issue| filter.is_empty() || !filter.is_empty() && filter.contains(&issue.key)).collect();
                debug!("Filtered {} issues for {}", issues.len(), &project.key);
                let _old = std::mem::replace(&mut project.issues, issues);
                for issue in &mut project.issues {
                    debug!("Retrieving worklogs for issue {}", &issue.key);
                    let key = issue.key.to_string();
                    let mut worklogs = get_worklogs_for(&client, key, started_after).await;
                    debug!("Issue {} has {} worklog entries", issue.key, worklogs.len());
                    issue.worklogs.append(&mut worklogs);
                }
                project
            })
        })
        .buffer_unordered(FUTURE_BUFFER_SIZE);

    let mut result = Vec::<JiraProject>::new();
    while let Some(r) = futures_stream.next().await {
        match r {
            Ok(jp) => {
                info!("Data retrieved from Jira for project {}", jp.key);
                result.push(jp);
            }
            Err(e) => eprintln!("Error: {:?}", e)
        }
    }
    result
}

pub fn midnight_a_month_ago_in() -> NaiveDateTime {
    let today = chrono::offset::Local::now();
    let a_month_ago = today.checked_sub_months(Months::new(1)).unwrap();
    NaiveDateTime::new(a_month_ago.date_naive(), NaiveTime::from_hms_opt(0, 0, 0).unwrap())
}


#[derive(Debug, Serialize, Deserialize)]
#[allow(non_snake_case)]
struct WorklogInsert {
    comment: String,
    started: String,
    timeSpentSeconds: i32,
}

pub async fn insert_worklog(http_client: &Client, issue_id: &str, started: DateTime<Local>, time_spent_seconds: i32, comment: &str) {
    // This is how Jira needs it.
    // Note! The formatting in Jira is based on the time zone of the user. Remember to change it
    // if you fly across the ocean :-)
    let start = started.format("%Y-%m-%dT%H:%M:%S.%3f%z");
    let worklog_entry = WorklogInsert {
        timeSpentSeconds: time_spent_seconds,
        comment: comment.to_string(),
        started: start.to_string(),
    };
    let json = serde_json::to_string(&worklog_entry).unwrap(); // Let Serde do the heavy lifting

    let url = format!("{}/issue/{}/worklog", JIRA_URL, issue_id);
    debug!("Composed url for worklog insert: {}", url);

    let result = http_client
        .post(url)
        .body(json)
        .header("Content-Type", "application/json")
        .send().await;

    match result {
        Ok(response) => { info!("Status {} = {} ", response.status(), response.text().await.unwrap()) }
        Err(e) => { panic!("Failed to insert {:?}", e) }
    }
}
