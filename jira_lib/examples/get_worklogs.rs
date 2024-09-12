use tokio;

use jira_lib::{JiraClient, midnight_a_month_ago_in};

#[tokio::main]
async fn main() {
    let jira_client = jira_lib::create_jira_client();

    let projects = jira_client.get_all_projects(vec![]).await;

    println!("Found {} projects", projects.len());
    for (i, project) in projects.iter().enumerate() {
        println!(
            "{:>3} {} {} {}, private={}",
            i, project.id, project.key, project.name, project.is_private
        );
    }
    let worklogs = JiraClient::get_worklogs_for(&jira_client.http_client,  "A3SRS-1".to_string(), midnight_a_month_ago_in()).await
        ;

    println!("{:?}", &worklogs);
}
