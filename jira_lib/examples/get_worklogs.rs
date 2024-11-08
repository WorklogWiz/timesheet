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

    let results = jira_client.get_worklogs_for_current_user("time-147", Option::None).await;
    if let Ok(worklogs) = results {
        for worklog in worklogs {
            println!("{} {} {}", worklog.id, worklog.started, worklog.timeSpent);
        }
    } else {
        println!("Unable to retrieve your worklogs for TIME-147");
    }
}
