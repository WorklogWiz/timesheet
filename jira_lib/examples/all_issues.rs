
use jira_lib::{get_issues_for_single_project, http_client, JiraIssue};

#[tokio::main]
async fn main() {
    // Creates HTTP client with all the required credentials
    let http_client = http_client();


    let results : Vec<JiraIssue> = get_issues_for_single_project(&http_client, "TIME".to_string()).await;
    for issue in results {
        println!("{}",issue.key);
    }

}