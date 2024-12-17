use chrono::{DateTime, Days, Local};
use log::debug;
use std::process::exit;

use jira::models::core::JiraKey;
use std::collections::BTreeMap;
use worklog::{date, error::WorklogError, storage::LocalWorklog, ApplicationRuntime};

use crate::{cli::Synchronisation, get_runtime};

pub async fn execute(sync: Synchronisation) -> Result<(), WorklogError> {
    let runtime = get_runtime();

    // Can we parse the string supplied by the user into a valid DateTime?
    let start_after = sync
        .started
        .clone()
        .map(|s| date::str_to_date_time(&s).unwrap());
    // If not, use a date 30 days back in time.
    let date_time = start_after.unwrap_or_else(|| {
        // Defaults to a month (approx)
        Local::now().checked_sub_days(Days::new(30)).unwrap()
    });

    let start_after_naive_date_time = DateTime::from_timestamp_millis(date_time.timestamp_millis())
        .unwrap()
        .naive_local();

    let issue_keys_to_sync = prepare_issue_keys_for_sync(&sync, &runtime).await?;

    println!("Synchronising work logs for these issues:");
    for issue in &issue_keys_to_sync {
        println!("\t{issue}");
    }
    debug!(
        "Synchronising with Jira for these issues {:?}",
        &issue_keys_to_sync
    );
    println!("Fetching work logs, this might take some time...");
    // Fetch all worklogs for all the specified issue keys
    let mut work_logs = runtime
        .jira_client()
        .fetch_work_logs_for_issues_concurrently(&issue_keys_to_sync, start_after_naive_date_time)
        .await?;
    // Filter for current user or all users
    if sync.all_users {
        eprintln!("Retrieving all work logs for all users");
    } else {
        let current_user = runtime.jira_client().get_current_user().await?;
        eprintln!(
            "Filtering work logs for current user: {:?} ",
            current_user.display_name
        );
        work_logs.retain(|wl| current_user.account_id == wl.author.accountId);
    }

    // Retrieve meta information for each issue key
    let keys: Vec<JiraKey> = issue_keys_to_sync
        .iter()
        .map(|s| JiraKey::from(s.as_str()))
        .collect();
    let issue_info = runtime.sync_jira_issue_information(&keys).await?;

    let issue_info_map: BTreeMap<_, _> = issue_info
        .iter()
        .map(|issue| (issue.id.clone(), issue))
        .collect();

    // Updates the database
    for worklog in work_logs {
        debug!("Removing and adding {:?}", &worklog);

        // Delete the existing one if it exists
        if let Err(e) = runtime.worklog_service().remove_entry(&worklog) {
            debug!("Unable to remove {:?}: {}", &worklog, e);
        }

        debug!("Adding {} {:?}", &worklog.issueId, &worklog);
        let issue = issue_info_map.get(&worklog.issueId).unwrap();
        let local_worklog = LocalWorklog::from_worklog(&worklog, JiraKey::new(issue.key.as_str()));
        if let Err(err) = runtime.worklog_service().add_entry(&local_worklog) {
            eprintln!(
                "Insert into database failed for {:?}, cause: {:?}",
                &local_worklog, err
            );
            exit(4);
        }
    }

    for issue in issue_info {
        println!("{:12} {}", issue.key, issue.fields.summary);
    }

    Ok(())
}

async fn prepare_issue_keys_for_sync(
    sync: &Synchronisation,
    runtime: &ApplicationRuntime,
) -> Result<Vec<JiraKey>, WorklogError> {
    let mut issue_keys_to_sync = sync
        .issues
        .iter()
        .map(|issue_key| JiraKey::new(issue_key))
        .collect::<Vec<JiraKey>>();

    if !sync.projects.is_empty() {
        let projects_as_str: Vec<&str> = sync
            .projects
            .iter()
            .map(std::string::String::as_str)
            .collect();
        println!(
            "Searching for issues in these projects: {:?}",
            &projects_as_str
        );
        let fetched_issue_keys = runtime
            .jira_client()
            .search_issues(&projects_as_str, &issue_keys_to_sync)
            .await?
            .iter()
            .map(|issue| issue.key.clone())
            .collect::<Vec<JiraKey>>();
        println!("Found {} issues", fetched_issue_keys.len());
        issue_keys_to_sync.extend(fetched_issue_keys);

        issue_keys_to_sync.sort();
        issue_keys_to_sync.dedup();
    }

    // If no projects and no issues were specified on the command line
    // have a look in the database and create a unique list from
    // entries in the past
    if issue_keys_to_sync.is_empty() && sync.projects.is_empty() {
        issue_keys_to_sync = runtime
            .worklog_service()
            .find_unique_keys()?
            .iter()
            .map(|k| JiraKey::new(k))
            .collect::<Vec<JiraKey>>();
    }
    if issue_keys_to_sync.is_empty() {
        eprintln!(
            "No issue keys to synchronise supplied on commandline or found in the local dbms"
        );
        exit(4);
    }
    Ok(issue_keys_to_sync)
}
