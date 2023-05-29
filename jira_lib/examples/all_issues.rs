
use jira_lib::{ JiraIssue};

#[tokio::main]
async fn main() {

    let jira_client = jira_lib::create_jira_client();


    let results : Vec<JiraIssue> = jira_client.get_issues_for_single_project("TIME".to_string()).await;
    for issue in results {
        println!("{}",issue.key);
    }

}