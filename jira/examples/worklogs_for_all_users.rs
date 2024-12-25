use chrono::{DateTime, Days, Local, NaiveDateTime};
use futures::StreamExt;
use jira::models::worklog::Worklog;
use jira::{Credentials, Jira};
use std::env;
use std::time::Instant;
use jira::models::issue::IssueSummary;

#[tokio::main]
async fn main() {
    env_logger::init();

    let jira = create_jira_client();

    let start_time = Instant::now();
    println!("Searching for issues, be patient this can take a while\n (minutes possibly, depending on the number of issues and the Jira instance you are using) ....");

    let issue_summaries = match jira.get_issue_summaries(&vec!["KT,PT"], &[], true ).await {
        Ok(issues) => issues,
        Err(e) => {
            eprintln!("Error searching issues: {}", e);
            return;
        }
    };
    let issue_fetch = start_time.elapsed();
    println!(
        "Found {} issues in {}s",
        issue_summaries.len(),
        issue_fetch.as_secs()
    );

    let date_time = Local::now().checked_sub_days(Days::new(30)).unwrap();
    let naive_date_time = DateTime::from_timestamp_millis(date_time.timestamp_millis())
        .unwrap()
        .naive_local();
    let start_worklogs = Instant::now();
    let work_logs = match fetch_worklogs_for_issues2(jira, issue_summaries, naive_date_time).await {
        Ok(logs) => logs,
        Err(e) => {
            eprintln!("Error searching issues for worklogs: {}", e);
            return;
        }
    };
    println!(
        "Fetched {} worklogs in {}s",
        work_logs.len(),
        start_worklogs.elapsed().as_secs()
    );
}

async fn fetch_worklogs_for_issues2(
    jira: Jira,
    issue_summaries: Vec<IssueSummary>,
    start_after: NaiveDateTime,
) -> Result<Vec<Worklog>, jira::JiraError> {
    let futures = issue_summaries.into_iter().map(|issue_summary| {
        let jira_client = jira.clone(); // Clone only once per async block
        async move {
            jira_client
                .get_work_logs_for_issue(&issue_summary.key, start_after)
                .await
        }
    });

    let results = futures::stream::iter(futures)
        .buffer_unordered(10) // Max 10 concurrent tasks
        .collect::<Vec<_>>()
        .await;

    let mut worklogs = Vec::new();
    for result in results {
        match result {
            Ok(logs) => worklogs.extend(logs),
            Err(err) => return Err(err), // Return the first error
        }
    }

    Ok(worklogs)
}

fn create_jira_client() -> Jira {
    if let (Ok(host), Ok(user), Ok(token)) = (
        env::var("JIRA_HOST"),
        env::var("JIRA_USER"),
        env::var("JIRA_TOKEN"),
    ) {
        let client = Jira::new(&host, Credentials::Basic(user, token))
            .expect("Error initializing jira client");
        return client;
    } else {
        println!("Please set JIRA_HOST, JIRA_USER and JIRA_TOKEN");
        std::process::exit(1);
    }
}
