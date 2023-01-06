use futures::StreamExt;
use tokio;

use jira;
use jira::{get_issues_for_project, get_worklogs_for, http_client, JiraProject};

#[tokio::main]
async fn main() {
    // Creates HTTP client with all the required credentials
    let http_client = http_client();

    let projects = jira::get_all_projects(&http_client).await;

    println!("Found {} projects", projects.len());
    for (i, project) in projects.iter().enumerate() {
        println!(
            "{:>3} {} {} {}, private={}",
            i, project.id, project.key, project.name, project.is_private
        );
    }
    let worklogs = get_worklogs_for(&http_client, "A3SRS-1").await
        ;

    println!("{:?}", &worklogs);
}
