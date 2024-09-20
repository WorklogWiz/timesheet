
#[tokio::main]
async fn main() {
    env_logger::init();

    let jira_client = jira_lib::create_jira_client();
    let issues = jira_client.get_issues_for_single_project("TIME".to_string()).await;
    for issue in issues {
        println!("{} {}", issue.key, issue.fields.summary);
    }

}
