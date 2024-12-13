use std::process::exit;

use log::debug;

use jira::models::core::JiraKey;
use worklog::{date, error::WorklogError, storage::LocalWorklog};

use crate::{cli::Synchronisation, get_runtime};

pub async fn execute(sync: Synchronisation) -> Result<(), WorklogError> {
    let runtime = get_runtime();
    let start_after = sync.started.map(|s| date::str_to_date_time(&s).unwrap());

    let mut issue_keys_to_sync = sync.issues.clone();
    if issue_keys_to_sync.is_empty() {
        issue_keys_to_sync = runtime.worklog_service().find_unique_keys()?;
    }
    if issue_keys_to_sync.is_empty() {
        eprintln!(
            "No issue keys to synchronise supplied on commandline or found in the local dbms"
        );
        exit(4);
    }

    println!("Synchronising work logs for these issues:");
    for issue in &issue_keys_to_sync {
        println!("\t{issue}");
    }
    debug!(
        "Synchronising with Jira for these issues {:?}",
        &issue_keys_to_sync
    );

    // Retrieve the work logs for each issue key specified on the command line
    for issue_key in &issue_keys_to_sync {
        let worklogs = runtime
            .jira_client()
            .get_worklogs_for_current_user(issue_key, start_after)
            .await
            .map_err(|e| WorklogError::JiraResponse {
                msg: format!("unable to get worklogs for current user {e}").to_string(),
                reason: e.to_string(),
            })?;
        // ... and insert them into our local data store
        println!(
            "Synchronising {} entries for time code {}",
            worklogs.len(),
            &issue_key
        );
        for worklog in worklogs {
            debug!("Removing and adding {:?}", &worklog);

            // Delete the existing one if it exists
            if let Err(e) = runtime.worklog_service().remove_entry(&worklog) {
                debug!("Unable to remove {:?}: {}", &worklog, e);
            }

            debug!("Adding {} {:?}", &issue_key, &worklog);

            let local_worklog =
                LocalWorklog::from_worklog(&worklog, JiraKey::from(issue_key.clone()));
            if let Err(err) = runtime.worklog_service().add_entry(&local_worklog) {
                eprintln!(
                    "Insert into database failed for {:?}, cause: {:?}",
                    &local_worklog, err
                );
                exit(4);
            }
        }
    }
    let keys: Vec<JiraKey> = issue_keys_to_sync
        .iter()
        .map(|s| JiraKey::from(s.as_str()))
        .collect();
    let issue_info = runtime.sync_jira_issue_information(&keys).await?;
    println!();
    for issue in issue_info {
        println!("{:12} {}", issue.key, issue.fields.summary);
    }

    Ok(())
}
