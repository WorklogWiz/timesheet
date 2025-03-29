use chrono::Local;
use jira::models::core::IssueKey;
use jira::models::issue::IssueSummary;
use jira::{Credentials, Jira};
use std::env;
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let host = env::var("JIRA_HOST")?;
    let user = env::var("JIRA_USER")?;
    let token = env::var("JIRA_TOKEN")?;

    let jira =
        Jira::new(&host, Credentials::Basic(user, token)).expect("Error initializing Jira client");

    println!("Searching for all issues with worklogs");
    let start_time = Instant::now();
    let issue_keys: Vec<IssueSummary> = jira
        .fetch_with_jql("worklogAuthor IS NOT EMPTY", vec!["key", "summary"])
        .await?;

    // let issue_keys = issue_keys_with_work_logs(jira.clone()).await?;
    println!(
        "Finished searching the issues in {:.2?}",
        start_time.elapsed()
    );

    let start_after = (Local::now() - chrono::Duration::days(30)).naive_local();

    println!("Fetching the worklogs for the first 2 issues");
    let start_fetch_two = Instant::now();
    let keys = issue_keys
        .iter()
        .map(|i| i.key.clone())
        .collect::<Vec<IssueKey>>();

    jira.chunked_work_logs(&keys.iter().take(2).cloned().collect(), start_after)
        .await?;
    println!(
        "Finished fetching worklogs for 2 issues in {:.2?}ms",
        start_fetch_two.elapsed().as_millis()
    );

    println!(
        "Fetching the worklogs for all ({})issues with startAfter={}",
        issue_keys.len(),
        start_after.and_utc().timestamp_millis()
    );
    let start_fetch_all = Instant::now();

    let final_result = jira.chunked_work_logs(&keys, start_after).await?;
    println!(
        "Finished fetching all the worklogs in {:.2?}",
        start_fetch_all.elapsed().as_millis()
    );
    assert!(!final_result.is_empty());
    println!("Found {} issues", final_result.len());

    println!("Found {} worklogs", final_result.len());
    println!("Total elapsed time {}ms", start_time.elapsed().as_millis());
    return Ok(());
}
