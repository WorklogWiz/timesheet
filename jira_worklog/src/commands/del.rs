use std::process::exit;

use reqwest::StatusCode;

use crate::{cli::Del, get_jira_client, get_runtime};

pub async fn execute(delete: &Del) {
    let runtime = get_runtime();
    let jira_client = get_jira_client(runtime.get_application_configuration());

    let current_user = jira_client.get_current_user().await;
    let worklog_entry = match jira_client
        .get_worklog(&delete.issue_id, &delete.worklog_id)
        .await
    {
        Ok(result) => result,
        Err(e) => match e {
            StatusCode::NOT_FOUND => {
                eprintln!(
                    "Worklog {} for issue '{}' not found",
                    &delete.worklog_id, &delete.issue_id
                );
                exit(4);
            }
            other => {
                eprintln!("ERROR: unknown http status code: {other}");
                exit(16)
            }
        },
    };

    if worklog_entry.author.accountId != current_user.account_id {
        eprintln!(
            "ERROR: You are not the owner of worklog with id {}",
            &delete.worklog_id
        );
        exit(403);
    }

    match jira_client
        .delete_worklog(delete.issue_id.clone(), delete.worklog_id.clone())
        .await
    {
        Ok(()) => println!("Jira work log id {} deleted from Jira", &delete.worklog_id),
        Err(e) => {
            println!("An error occurred, worklog entry probably not deleted: {e}");
            exit(4);
        }
    }
    match runtime
        .get_local_worklog_service()
        .remove_entry_by_worklog_id(delete.worklog_id.as_str())
    {
        Ok(()) => {
            println!("Removed entry {} from local worklog", delete.worklog_id);
        }
        Err(err) => {
            panic!(
                "Deletion from local worklog failed for worklog.id = '{}' : {err}",
                delete.worklog_id.as_str()
            );
        }
    }
}
