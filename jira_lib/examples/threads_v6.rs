use env_logger::Env;
use futures::{Future, stream, Stream, StreamExt};
use lazy_static::lazy_static;
use reqwest::Client;
use tokio::time::Instant;
use jira_lib::{JiraClient, JiraIssue, midnight_a_month_ago_in, Worklog};

lazy_static! {
    static ref START_TIME: Instant = Instant::now();
}

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    let jira_client = jira_lib::create_jira_client();
    let http_client= jira_client.http_client;


    let start = Instant::now();
    let projects = jira_client.get_all_projects( vec![]).await;
    let _elapsed = start.elapsed().as_millis();

    let project_keys = projects.iter().map(|p| p.key.to_string()).collect();
    let start = Instant::now();
    let issues = get_all_the_bloody_issues(&jira_client, project_keys).await;
    println!("Retrieved {} issues in {}ms", issues.len(), start.elapsed().as_millis());

    let issue_keys = issues.iter().map(|i| i.key.to_string()).collect();
    let start = Instant::now();
    let logs = get_all_worklogs(&http_client, issue_keys).await;
    println!("Retrieved {} worklogs in {}ms", logs.len(), start.elapsed().as_millis());

}


async fn get_all_the_bloody_issues(jira_client: &JiraClient, project_keys : Vec<String> ) -> Vec<JiraIssue> {
    let futures_result: Vec<Vec<JiraIssue>> = stream::iter(project_keys).map(|p| {

        jira_client.get_issues_for_single_project(p)
    }).buffer_unordered(30).collect().await;

    futures_result.into_iter().flatten().collect()
}

async fn get_all_worklogs(http_client: &Client, issue_keys: Vec<String>) -> Vec<Worklog> {
    let result: Vec<Vec<Worklog>> = stream::iter(issue_keys)
        .map(|key| {
            JiraClient::get_worklogs_for(http_client,key, midnight_a_month_ago_in())
        }).buffer_unordered(30).collect().await;

    result.into_iter().flatten().collect()
}



/// Deprecated, left to illustrate alternative usage of futures
async fn _process_issue_worklogs(http_client: &Client, issues: Vec<String>, _users: Option<Vec<String>>) {
    let worklogs = execute_worklogs_futures(http_client, issues).await;
    println!("Found {} worklog entries", worklogs.len());
}

#[allow(dead_code)]
async fn execute_worklogs_futures(http_client: &Client, issues: Vec<String>) -> Vec<Worklog> {
    let result: Vec<Vec<Worklog>> = worklogs_stream(http_client, issues).buffer_unordered(10).collect().await;
    result.into_iter().flatten().collect()
}
#[allow(dead_code)]
fn worklogs_stream(http_client: &Client, issues: Vec<String>) -> impl Stream<Item=impl Future<Output=Vec<Worklog>> + '_> + '_{
    stream::iter(issues).map(move |issue| {
        get_worklogs_for(&http_client, issue,midnight_a_month_ago_in() )
    })
}
