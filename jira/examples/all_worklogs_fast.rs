use futures::{stream, StreamExt};
use jira::models::core::{JiraFields, JiraKey};
use jira::models::worklog::Worklog;
use jira::{Credentials, Jira};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Deserializer, Serialize};
use std::env;
use std::time::Instant;

#[derive(Debug, Serialize)]
struct IssuesResponse<T>
where
    T: DeserializeOwned,
{
    issues: Vec<T>,
    #[serde(rename = "nextPageToken")] // Ensure field matches the JSON representation
    next_page_token: Option<String>,
}

impl<T> IssuesResponse<T> where T: DeserializeOwned {}

// Manually implement `Deserialize` for `IssuesResponse<T>`
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
#[derive(Debug, Serialize, Deserialize)]
struct IssueSummary {
    expand: String,
    id: String,
    #[serde(rename = "self")]
    self_url: String,
    key: JiraKey,
    fields: JiraFields,
}

#[derive(Debug, Serialize, Deserialize)]
struct IssuesWorklogsResult {
    issues: Vec<IssueSummaryAndWorklog>,
    #[serde(rename = "nextPageToken")] // Ensure field matches the JSON representation
    next_page_token: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct IssueSummaryAndWorklog {
    id: String,
    key: JiraKey,
    fields: FieldsWithWorklog,
}

#[derive(Debug, Serialize, Deserialize)]
struct FieldsWithWorklog {
    summary: String,
    // components
    worklog: Worklogs,
}
#[derive(Debug, Serialize, Deserialize)]
struct Worklogs {
    worklogs: Vec<Worklog>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let host = env::var("JIRA_HOST")?;
    let user = env::var("JIRA_USER")?;
    let token = env::var("JIRA_TOKEN")?;

    let jira =
        Jira::new(&host, Credentials::Basic(user, token)).expect("Error initializing Jira client");

    println!("Searching for all issues with worklogs");
    let start_time = Instant::now();
    let issue_keys = fetch_all_issue_keys(jira.clone()).await?;
    println!(
        "Finished searching the issues in {:.2?}",
        start_time.elapsed()
    );

    println!("Fetching the worklogs for the first 2 issues");
    let start_fetch_two = Instant::now();
    let keys = issue_keys.iter().take(2).cloned().collect::<Vec<JiraKey>>();
    let issues_with_work_logs = fetch_work_logs_for_keys(jira.clone(), keys).await?;
    for issue in issues_with_work_logs {
        println!("Issue: {} {}", issue.key, issue.fields.summary);
        println!("Summary: {}", issue.fields.summary);
    }
    println!(
        "Finished fetching 2 worklogs in {:.2?}",
        start_fetch_two.elapsed().as_millis()
    );

    println!("Fetching the worklogs for all issues");
    let start_fetch_all = Instant::now();
    let final_result = process_keys(jira, issue_keys).await?;
    println!(
        "Finished fetching all the worklogs in {:.2?}",
        start_fetch_all.elapsed().as_millis()
    );
    assert!(final_result.len() > 0);
    println!("Found {} issues", final_result.len());

    let mut counter = 0;
    for issue in final_result {
        counter += issue.fields.worklog.worklogs.len();
    }
    println!("Found {} worklogs", counter);
    println!("Total elapsed time {}ms", start_time.elapsed().as_millis());
    return Ok(());
}

fn group_issue_keys(issue_keys: Vec<JiraKey>, n: usize) -> Vec<Vec<JiraKey>> {
    issue_keys
        .chunks(n) // Create chunks of size `n`
        .map(|chunk| chunk.to_vec()) // Convert each chunk slice into a Vec
        .collect() // Collect into a Vec<Vec<JiraKey>>
}
async fn process_keys(
    jira: Jira,
    issue_keys: Vec<JiraKey>,
) -> Result<Vec<IssueSummaryAndWorklog>, Box<dyn std::error::Error>> {
    let issue_keys = group_issue_keys(issue_keys, 10);
    println!("Fetching {} chunks", issue_keys.len());
    let futures = stream::iter(issue_keys)
        .map(|key| {
            let jira = jira.clone();
            fetch_work_logs_for_keys(jira, key)
        })
        .buffer_unordered(10);

    let worklogs: Vec<_> = futures
        .filter_map(|result| async {
            match result {
                Ok(worklogs) => Some(worklogs),
                Err(_) => None,
            }
        })
        .concat()
        .await;

    Ok(worklogs)
}

const MAX_RESULTS: i32 = 100;
async fn fetch_work_logs_for_keys(
    jira: Jira,
    issue_keys: Vec<JiraKey>,
) -> Result<Vec<IssueSummaryAndWorklog>, Box<dyn std::error::Error>> {

    let key_string = issue_keys
        .iter()
        .map(|key| key.to_string())
        .collect::<Vec<String>>()
        .join(",");

    let jql = &format!("key in ({}) and worklogAuthor is not empty", key_string);

    Ok(fetch_issues_from_jql::<IssueSummaryAndWorklog>(jira, jql, vec!["key", "summary", "components", "statusCategory", "worklog"]).await?)

/*    let jql_encoded = urlencoding::encode(jql);

    let mut next_page_token = None;

    loop {
        let url = if let Some(token) = next_page_token {
            format!("/search/jql?jql={}&fields=key,summary,components,statusCategory,worklog&maxResults={}&nextPageToken={}", jql_encoded, MAX_RESULTS, token)
        } else {
            format!("/search/jql?jql={}&fields=key,summary,components,statusCategory,worklog&maxResults={}", jql_encoded, MAX_RESULTS)
        };

        let result: IssuesWorklogsResult = jira.get(&url).await?;
        issue_worklogs.extend(result.issues.into_iter());
        if let Some(token) = result.next_page_token {
            println!("Found more issues, continuing... {token}");
            next_page_token = Some(token);
        } else {
            break;
        }
    }
*/
}

async fn fetch_issues_from_jql<T>(jira: Jira, jql: &str, fields: Vec<&str>) -> Result<Vec<T>, Box<dyn std::error::Error>>
where T: DeserializeOwned,
{
    let jql_encoded = urlencoding::encode(jql);
    let mut results: Vec<T> = Vec::new();

    let mut next_page_token = None;
    loop {

        let resource= if let Some(token) = next_page_token {
            format!("/search/jql?jql={}&fields={}&maxResults={}&nextPageToken={}", jql_encoded, fields.join(","), MAX_RESULTS, token)
        } else {
            format!("/search/jql?jql={}&fields={}&maxResults={}", jql_encoded, fields.join(","), MAX_RESULTS)
        };

        let response: IssuesResponse<T> = jira.get(&resource).await?;
        results.extend(response.issues);

        if let Some(token) = response.next_page_token {
            println!("Found more issues, continuing... {token}");
            next_page_token = Some(token);
        } else {
            break;
        }
    }
    return Ok(results);
}

async fn fetch_all_issue_keys(jira: Jira) -> Result<Vec<JiraKey>, Box<dyn std::error::Error>> {
    let jql_encoded = urlencoding::encode("project in (PT,KT) and worklogAuthor is not empty");

    let mut next_page_token = None;
    let mut all_issue_keys = Vec::new();

    const MAX_RESULTS: i32 = 100;
    loop {
        let url = if let Some(token) = next_page_token {
            format!(
                "/search/jql?jql={}&fields=key,summary&maxResults={}&nextPageToken={}",
                jql_encoded, MAX_RESULTS, token
            )
        } else {
            format!(
                "/search/jql?jql={}&fields=key,summary&maxResults={}",
                jql_encoded, MAX_RESULTS
            )
        };

        let results: IssuesResponse<IssueSummary> = jira.get(&url).await?;

        all_issue_keys.extend(results.issues.iter().map(|issue| issue.key.clone()));

        println!(
            "Found {} issues for projects PT and KT",
            results.issues.len()
        );

        if let Some(token) = results.next_page_token {
            println!("Found more issues, continuing... {token}");
            next_page_token = Some(token);
        } else {
            break;
        }
    }
    Ok(all_issue_keys)
}
