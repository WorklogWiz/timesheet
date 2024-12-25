use chrono::{DateTime, Days, Local};
use log::debug;
use std::process::exit;

use jira::models::core::IssueKey;
use jira::models::issue::IssueSummary;
use worklog::{date, error::WorklogError, storage::LocalWorklog, ApplicationRuntime};

use crate::{cli::Synchronisation, get_runtime};

fn get_default_start_date() -> DateTime<Local> {
    Local::now()
        .checked_sub_days(Days::new(30))
        .expect("Failed to create default fallback date")
}

pub async fn execute(sync_cmd: Synchronisation) -> Result<(), WorklogError> {
    let runtime = get_runtime();

    // Parse the start date or fall back to the default
    let date_time = sync_cmd
        .started
        .as_deref()
        .and_then(|s| date::str_to_date_time(s).ok())
        .unwrap_or_else(get_default_start_date);

    let start_after_naive_date_time = DateTime::from_timestamp_millis(date_time.timestamp_millis())
        .expect("Invalid timestamp")
        .naive_local();

    let issue_summaries = prepare_issue_keys_for_sync(&sync_cmd, &runtime).await?;
    if issue_summaries.is_empty() {
        eprintln!(
            "No issue keys to synchronise supplied on commandline or found in the local dbms"
        );
        exit(4);
    }

    println!("Synchronising work logs for these issues:");
    for issue in &issue_summaries {
        println!("\t{:8} {}", issue.key, issue.fields.summary);
    }
    debug!(
        "Synchronising with Jira for these issues {:?}",
        &issue_summaries
    );

    println!("Fetching work logs, this might take some time...");
    // Fetch all worklogs for all the specified issue keys
    let mut all_issue_work_logs = runtime
        .jira_client()
        .chunked_work_logs(
            &issue_summaries.iter().map(|s| s.key.clone()).collect(),
            start_after_naive_date_time,
        )
        .await?;

    // Filter for current user or all users
    if sync_cmd.all_users {
        eprintln!("Retrieving all work logs for all users");
    } else {
        let current_user = runtime.jira_client().get_current_user().await?;
        eprintln!(
            "Filtering work logs for current user: {:?} ",
            current_user.display_name
        );
        all_issue_work_logs.retain(|wl| current_user.account_id == wl.author.accountId);
    }

    eprintln!("Found {} work logs", all_issue_work_logs.len());

    // Updates the database with the issue summary information
    runtime.sync_jira_issue_information(&issue_summaries)?;

    let issue_map: std::collections::HashMap<String, &IssueSummary> = issue_summaries
        .iter()
        .map(|issue| (issue.id.clone(), issue))
        .collect();

    // Inserts the work log entries into the database
    for worklog in &all_issue_work_logs {
        debug!("Removing and adding {:?}", &worklog);

        // Delete the existing one if it exists
        if let Err(e) = runtime.worklog_service().remove_entry(worklog) {
            debug!("Unable to remove {:?}: {}", &worklog, e);
        }

        debug!("Adding {} {:?}", &worklog.issueId, &worklog);

        let issue_summary = issue_map.get(&worklog.issueId).unwrap();
        let local_worklog = LocalWorklog::from_worklog(worklog, &issue_summary.key);
        if let Err(err) = runtime.worklog_service().add_entry(&local_worklog) {
            eprintln!(
                "Insert into database failed for {:?}, cause: {:?}",
                &local_worklog, err
            );
            exit(4);
        }
    }

    Ok(())
}

/// Helper function to transform a list of strings into a list of `IssueKey`s
fn collect_issue_keys(issue_strings: &[String]) -> Vec<IssueKey> {
    issue_strings
        .iter()
        .map(|s| IssueKey::from(s.as_str()))
        .collect()
}

async fn prepare_issue_keys_for_sync(
    sync_cmd: &Synchronisation,
    runtime: &ApplicationRuntime,
) -> Result<Vec<IssueSummary>, WorklogError> {
    // Transform from list of strings to list of IssueKey
    let mut issue_keys_to_sync = collect_issue_keys(&sync_cmd.issues);

    // If no projects and no issues were specified on the command line
    // have a look in the database and create a unique list from
    // entries in the past
    if issue_keys_to_sync.is_empty() && sync_cmd.projects.is_empty() {
        issue_keys_to_sync = runtime
            .worklog_service()
            .find_unique_keys()?
            .iter()
            .map(|k| IssueKey::new(k))
            .collect::<Vec<IssueKey>>();
    }

    let projects_as_str: Vec<&str> = sync_cmd.projects.iter().map(String::as_str).collect();
    println!(
        "Searching for issues in these projects: {:?}",
        &projects_as_str
    );

    // Gets the Issue Summaries for all the filter options specified on the command line
    let mut issue_keys_to_sync = runtime
        .jira_client()
        .get_issue_summaries(&projects_as_str, &issue_keys_to_sync, sync_cmd.all_users)
        .await?;

    println!("Resolved {} issues", issue_keys_to_sync.len());

    issue_keys_to_sync.sort();
    issue_keys_to_sync.dedup();

    Ok(issue_keys_to_sync)
}
