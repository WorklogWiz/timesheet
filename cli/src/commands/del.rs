//! Delete command - deletes worklogs from both local DB and Jira

use log::info;
use worklog_core::WorklogError;

use crate::services::Services;

pub struct DeleteOptions {
    pub issue_key: String,
    pub worklog_id: String,
}

/// Execute delete command - deletes a worklog from both local DB and Jira
pub async fn execute(services: &Services, options: DeleteOptions) -> Result<(), WorklogError> {
    // Find the local entry by provider_worklog_id
    let worklogs = services.worklog.find_by_issue(&options.issue_key).await?;

    let entry = worklogs
        .iter()
        .find(|w| {
            w.provider_worklog_id
                .as_ref()
                .is_some_and(|id| id == &options.worklog_id)
        })
        .ok_or_else(|| {
            WorklogError::EntryNotFound(format!(
                "Worklog {} not found locally for issue {}",
                options.worklog_id, options.issue_key
            ))
        })?;

    let local_id = entry
        .id
        .as_ref()
        .ok_or_else(|| WorklogError::EntryNotFound("Worklog has no local ID".to_string()))?;

    // Delete using the service (handles both local and remote)
    services.worklog.delete_and_sync(local_id).await?;

    info!(
        "Deleted worklog {} from both local DB and Jira",
        options.worklog_id
    );

    Ok(())
}
