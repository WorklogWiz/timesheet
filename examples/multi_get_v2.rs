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

    // Only retrieve data for Jira projects which are not private
    let filtered_projects: Vec<JiraProject> = projects
        .into_iter()
        .filter(|p| p.is_private != true)
        .collect();

    let bodies = futures::stream::iter(filtered_projects)
        .map(|mut project| {
            let client = http_client.clone();
            tokio::spawn(async move {
                let issues = get_issues_for_project(&client, &project.key).await;
                let _old = std::mem::replace(&mut project.issues, issues);
                for issue in &mut project.issues {
                    println!("Retrieving worklogs for {}", &issue.key);
                    let mut worklogs = get_worklogs_for(&client, &issue.key).await;
                    println!("Issue {} has {} worklog entries", issue.key, worklogs.len());
                    issue.worklogs.append(&mut worklogs);
                }
                project
            })
        })
        .buffer_unordered(10);

    println!("I have started looping in for_each()..");

    let mut results = Vec::<JiraProject>::new();
    bodies.take(10)
        .for_each(|result| async move {
            match result {
                Ok(jp) => {
                    println!("-- project {:>5} with issues {} ", jp.key, jp.issues.len());
                }
                Err(e) => eprintln!("Ouch, a real error {:?}", e),
            }
        })
        .await;
}
