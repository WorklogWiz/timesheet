use futures::StreamExt;
use tokio;

use jira;
use jira::{get_issues_for_single_project, get_worklogs_for, http_client, JiraProject, midnight_a_month_ago_in};

#[tokio::main]
async fn main() {
    // Creates HTTP client with all the required credentials
    let http_client = http_client();

    let projects = jira::get_all_projects(&http_client, vec![]).await;

    println!("Found {} projects", projects.len());
    for (i, project) in projects.iter().enumerate() {
        println!(
            "{:>3} {} {} {}, private={}",
            i, project.id, project.key, project.name, project.is_private
        );
    }
    let worklogs = get_worklogs_for(&http_client, "A3SRS-1".to_string(), midnight_a_month_ago_in()).await
        ;

    println!("{:?}", &worklogs);
}
