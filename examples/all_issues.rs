use tokio::task::JoinError;
use jira;
use jira::{get_worklogs_for, http_client, JiraProjectsPage, JiraProject, JiraIssue, get_issues_for_project};
use jira::WorklogsPage;

#[tokio::main]
async fn main() {
    // Creates HTTP client with all the required credentials
    let http_client = http_client();

    let projects = get_jira_resource::<JiraProjectsPage>(&http_client, "/project/search?maxResults=50&startAt=0").await;

    println!("Projects startAt: {}, maxResults: {} of total: {}", projects.startAt, projects.maxResults, projects.total.unwrap());

    for (i, project) in projects.values.iter().enumerate() {
        println!("{} {} {} {}", i, project.id, project.key, project.name);
    }

    let results : Vec<JiraIssue> = get_issues_for_project(&http_client, "TIME").await;
    for issue in results {
        println!("{}",issue.key);
    }

}