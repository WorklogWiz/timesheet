use tokio::task::JoinError;
use jira;
use jira::{get_worklogs_for, http_client, JiraProjectsPage, JiraProject, JiraIssue, get_issues_for_single_project};
use jira::WorklogsPage;

#[tokio::main]
async fn main() {
    // Creates HTTP client with all the required credentials
    let http_client = http_client();


    let results : Vec<JiraIssue> = get_issues_for_single_project(&http_client, "TIME".to_string()).await;
    for issue in results {
        println!("{}",issue.key);
    }

}