extern crate core;

pub mod dbms;

use chrono::{Datelike, DateTime, NaiveDate, Utc};
use reqwest::header::HeaderMap;
use reqwest::header::HeaderValue;
use reqwest::Client;
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde::Serialize;
use std::time::Instant;
use futures::{StreamExt};

pub const JIRA_URL: &str = "https://autostore.atlassian.net/rest/api/latest";

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
    pub id: String,             // "557058:189520f0-d1fb-4a0d-b555-bc44ec1f4ebc"
    pub author: Author,
    pub created: DateTime<Utc>,
    pub updated: DateTime<Utc>,
    pub started: DateTime<Utc>,
    pub timeSpent: String,
    pub timeSpentSeconds: i32,
    pub issueId: String,
}

#[derive(Debug, Deserialize, Serialize, PartialOrd, PartialEq,Eq, Hash, Clone)]
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
    pub key: String,
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
}

pub fn http_client() -> reqwest::Client {
    create_auth_value();

    let client = match reqwest::Client::builder()
        .default_headers(create_default_headers())
        .build()
    {
        Ok(c) => c,
        Err(error) => panic!("Unable to create http client {:?}", error),
    };
    client
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

pub async fn get_projects_filtered(http_client: &Client, filter_projects: Option<Vec<String>>) -> Vec<JiraProject> {

    let projects = get_all_projects(&http_client).await;
    if let Some(filter_keys) = filter_projects {
        projects.into_iter().filter(|p| filter_keys.contains(&p.key)).collect()
    } else {
        projects
    }
}

/// Retrieves all Jira projects, filtering out the private ones
pub async fn get_all_projects(http_client: &Client) -> Vec<JiraProject> {
    let start_at = 0;

    // Retrieves first page of Jira projects
    let mut project_page =
        get_jira_resource::<JiraProjectsPage>(&http_client, &project_search_resource(start_at))
            .await;

    let mut projects = Vec::<JiraProject>::new();
    if project_page.values.len() == 0 {
        // No projects, just return empty vector
        return projects;
    }

    projects.append(&mut project_page.values);

    // While there is a URL for the next page ...
    while let Some(url) = &project_page.nextPage {
        // Fetch next page of data
        project_page = get_jira_data_from_url::<JiraProjectsPage>(&http_client, &url).await;
        // Filter out the private projects and append to our list of projects
        projects.append(&mut project_page.values.into_iter().filter(|p| !p.is_private).collect());
    }
    projects
}

// TODO: Consider Trait with associated type as this logic is identical to get_worklogs_for()
pub async fn get_issues_for_project(http_client: &Client, project_key: &str) -> Vec<JiraIssue> {
    let mut resource = compose_resource_and_params(project_key, 0, 1024);

    let mut issues = Vec::<JiraIssue>::new();
    loop {
        let mut issue_page = get_jira_resource::<JiraIssuesPage>(http_client, &resource).await;
        // issues.len() will be invalid once we move the contents of the issues into our result
        let is_last_page = issue_page.issues.len() < issue_page.max_results as usize;
        if !is_last_page {
            resource = compose_resource_and_params(
                project_key,
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
pub async fn get_worklogs_for(http_client: &Client, issue_key: &str) -> Vec<Worklog> {
    let mut resource_name = compose_worklogs_url(issue_key, 0, 1024);
    let mut worklogs: Vec<Worklog> = Vec::<Worklog>::new();
    loop {
        println!("Retrieving worklogs for {}", issue_key);
        let mut worklog_page = get_jira_resource::<WorklogsPage>(http_client, &resource_name).await;
        let is_last_page = worklog_page.worklogs.len() < worklog_page.max_results as usize;
        if !is_last_page {
            resource_name = compose_worklogs_url(
                issue_key,
                worklog_page.startAt + worklog_page.worklogs.len() as i32,
                worklog_page.max_results,
            );
        }
        worklogs.append(&mut worklog_page.worklogs);
        if is_last_page {
            break;
        }
    }
    worklogs
}

fn compose_worklogs_url(issue_key: &str, start_at: i32, max_results: i32) -> String {
    let today = chrono::offset::Utc::now();
    let uxtime_this_year = NaiveDate::from_ymd_opt(today.year() - 1, today.month(), today.day())
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap()
        .timestamp_millis();
    format!(
        "/issue/{}/worklog?startAt={}&maxResults={}&startedAfter={}",
        issue_key, start_at, max_results, uxtime_this_year
    )
}

fn compose_resource_and_params(project_key: &str, start_at: i32, max_results: i32) -> String {
    let jql = format!("project=\"{}\" and resolution=Unresolved", project_key);
    let jql_encoded = urlencoding::encode(&jql);
    let resource = format!(
        "/search?jql={}&startAt={}&maxResults={}&fields={}",
        jql_encoded, start_at, max_results, "summary"
    );
    resource
}

async fn get_jira_resource<T: DeserializeOwned>(
    http_client: &Client,
    rest_resource: &str,
) -> T {
    let url = format!("{}{}", JIRA_URL, rest_resource);

    get_jira_data_from_url::<T>(http_client, &url).await
}

pub async fn get_jira_data_from_url<T: DeserializeOwned>(http_client: &Client, url: &str) -> T {
    let url_decoded = urlencoding::decode(&url).unwrap();

    println!("http get {}\n\t{}", url, url_decoded);
    let start = Instant::now();
    let response = http_client.get(url.clone()).send().await.unwrap();
    let elapsed = start.elapsed();
    println!("{} took {}ms", url_decoded, elapsed.as_millis());
    // Downloads the entire body of the response and convert from JSON to type safe struct
    let typed_result: T = match response.status() {
        reqwest::StatusCode::OK => {
            // Transforms JSON in body to type safe struct
            match response.json::<T>().await {
                Ok(wl) => {
                    println!("{}", start.elapsed().as_millis());
                    wl
                }, // Everything OK, return the Worklogs struct
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

fn project_search_resource<'a>(start_at: i32) -> String {
    // Seems 50 is the max value of maxResults
    format!("/project/search?maxResults=50&startAt={}", start_at)
}

pub async fn get_issues_for_projects(http_client: &Client, projects: Vec<JiraProject>) -> Vec<JiraProject> {
    let mut futures_stream = futures::stream::iter(projects)
        .map(|mut project| {
            let client = http_client.clone();
            tokio::spawn(async move {
                let issues = get_issues_for_project(&client, &project.key).await;
                let _old = std::mem::replace(&mut project.issues, issues);
                project
            })
        }).buffer_unordered(10);

    let mut result = Vec::<JiraProject>::new();
    while let Some(r) = futures_stream.next().await {
        match r {
            Ok(jp) => {
                println!("OK {}", jp.key);
                result.push(jp);
            },
            Err(e) => eprintln!("Error: {:?}", e)
        }
    }
    result
}

pub async fn get_issues_and_worklogs(http_client: &Client, projects: Vec<JiraProject>) -> Vec<JiraProject> {
    let mut bodies  = futures::stream::iter(projects)
        .map(|mut project| {
            let client = http_client.clone();
            println!("Creating future for {}", &project.key);
            tokio::spawn(async move {
                let issues = get_issues_for_project(&client, &project.key).await;
                let _old = std::mem::replace(&mut project.issues, issues);
                for  issue in &mut project.issues {
                    println!("Retrieving worklogs for issue {}", &issue.key);
                    let mut worklogs = get_worklogs_for(&client, &issue.key).await;
                    println!("Issue {} has {} worklog entries", issue.key, worklogs.len());
                    issue.worklogs.append(&mut worklogs);
                }
                project
            })
        })
        .buffer_unordered(10);


    let mut result = Vec::<JiraProject>::new();
    while let Some(r) = bodies.next().await {
        match r {
            Ok(jp) => {
                println!("OK {}", jp.key);
                result.push(jp);
            },
            Err(e) => eprintln!("Error: {:?}", e)
        }
    }
    result
}
