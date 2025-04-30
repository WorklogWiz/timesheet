use chrono::{DateTime, Days, Local};
use log::debug;
use std::process::exit;

use crate::error::WorklogError;
use crate::types::LocalWorklog;
use crate::{date, ApplicationRuntime};
use jira::models::core::IssueKey;
use jira::models::issue::IssueSummary;

pub struct Sync {
    pub started: Option<String>,
    pub all_users: bool,
    pub projects: Vec<String>,
    pub issues: Vec<String>,
}

/// Executes the main synchronization logic for work logs with Jira.
///
/// This function performs the following tasks:
/// - Parses the start date from the provided `sync_cmd` structure, or falls back to a default date.
/// - Prepares issue keys for synchronization by resolving them from the command-line input or the local database.
/// - Logs and outputs the list of issues being synchronized.
/// - Fetches work log entries from Jira for the specified issues and filters them based on the synchronization options.
/// - Updates the local database with issue summary information and inserts the fetched work logs.
///
/// # Arguments
/// * `runtime` - The application runtime that provides access to services, including Jira and the worklog database.
/// * `sync_cmd` - The synchronization command containing options like start date, projects, issues, and user settings.
///
/// # Returns
/// * `Result<(), WorklogError>` - Returns `Ok(())` on successful execution, or a `WorklogError` if any error occurs.
///
/// # Errors
/// This function will return an error if:
/// - The date parsing fails.
/// - Work log retrieval or filtering encounters an issue.
/// - Database operations, like adding or removing entries, fail.
///
///
/// # Panics
/// This function will panic if:
/// - The `timestamp` derived from the start date is invalid while creating a `DateTime`.
/// - The calculation of the default start date (30 days ago) fails.
///
/// These panics are due to calls to `expect` when creating `DateTime` or during date manipulation.
/// Ensure the input data is valid and that system date/time functionality behaves as expected.
/// # Behavior
/// If no issues are found, the function will print an error message and exit with a status code of 4.
/// The function uses debugging logs to trace execution details.
pub async fn execute(runtime: &ApplicationRuntime, sync_cmd: &Sync) -> Result<(), WorklogError> {
    let current_user = runtime.jira_client().get_current_user().await?;
    runtime
        .user_service()
        .insert_or_update_current_user(&current_user)?;

    // Parse the start date or fall back to the default
    let date_time = sync_cmd
        .started
        .as_deref()
        .and_then(|s| date::str_to_date_time(s).ok())
        .unwrap_or_else(get_default_start_date);

    let start_after_naive_date_time = DateTime::from_timestamp_millis(date_time.timestamp_millis())
        .expect("Invalid timestamp")
        .naive_local();

    let issue_summaries = prepare_issue_keys_for_sync(sync_cmd, runtime).await?;
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
        eprintln!("Synchronising work logs for all users");
    } else {
        eprintln!(
            "Filtering work logs for current user: {:?} ",
            current_user.display_name
        );
        all_issue_work_logs.retain(|wl| current_user.account_id == wl.author.accountId);
    }

    eprintln!("Found {} work logs", all_issue_work_logs.len());

    // Updates the database with the issue summary information
    sync_jira_issue_information(runtime, &issue_summaries)?;

    eprintln!("Updated database with issue summary information");
    // Create map of IssueKey -> IssueSummary
    let issue_map: std::collections::HashMap<String, &IssueSummary> = issue_summaries
        .iter()
        .map(|issue| (issue.id.clone(), issue))
        .collect();

    // Inserts the work log entries into the database
    for worklog in &all_issue_work_logs {
        debug!("Removing and adding {:?}", &worklog);

        // Delete the existing one if it exists
        if let Err(e) = runtime.worklog_service().remove_worklog_entry(worklog) {
            debug!("Unable to remove {:?}: {}", &worklog, e);
        }

        debug!("Adding {} {:?}", &worklog.issueId, &worklog);

        let issue_summary = issue_map.get(&worklog.issueId).unwrap();
        let local_worklog = LocalWorklog::from_worklog(worklog, &issue_summary.key);
        if let Err(err) = runtime.worklog_service().add_entry(&local_worklog).await {
            eprintln!(
                "Insert into database failed for {:?}, cause: {:?}",
                &local_worklog, err
            );
            exit(4);
        }
    }

    Ok(())
}

fn get_default_start_date() -> DateTime<Local> {
    Local::now()
        .checked_sub_days(Days::new(30))
        .expect("Failed to create default fallback date")
}

/// Helper function to transform a list of strings into a list of `IssueKey`s
fn collect_issue_keys(issue_strings: &[String]) -> Vec<IssueKey> {
    issue_strings
        .iter()
        .map(|s| IssueKey::from(s.as_str()))
        .collect()
}

async fn prepare_issue_keys_for_sync(
    sync_cmd: &Sync,
    runtime: &ApplicationRuntime,
) -> Result<Vec<IssueSummary>, WorklogError> {
    // Transform from list of strings to list of IssueKey
    let mut issue_keys_to_sync = collect_issue_keys(&sync_cmd.issues);

    // If no projects and no issues were specified on the command line
    // have a look in the database and create a unique list from
    // entries in the past
    if issue_keys_to_sync.is_empty() && sync_cmd.projects.is_empty() {
        issue_keys_to_sync = runtime.issue_service().find_unique_keys()?;
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

#[allow(clippy::missing_errors_doc)]
fn sync_jira_issue_information(
    runtime: &ApplicationRuntime,
    issue_summaries: &Vec<IssueSummary>,
) -> Result<(), WorklogError> {
    debug!("Searching for Jira issues (information)...");

    runtime.issue_service().add_jira_issues(issue_summaries)?;
    debug!("sync_jira_issue_information: add_jira_issues() done");
    for issue in issue_summaries {
        runtime
            .component_service()
            .create_component(&issue.key, &issue.fields.components)?;
    }
    debug!("sync_jira_issue_information: done");
    Ok(())
}
