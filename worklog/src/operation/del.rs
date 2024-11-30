use crate::{error::WorklogError, ApplicationRuntime};

pub struct Del {
    pub issue_id: String,
    pub worklog_id: String,
}

pub(crate) async fn execute(runtime: ApplicationRuntime, instructions: &Del) -> Result<String, WorklogError> {
    let client = runtime.jira_client();

    let current_user = client.get_current_user().await?;
    let worklog_entry = client
        .get_worklog(&instructions.issue_id, &instructions.worklog_id)
        .await?;

    if worklog_entry.author.accountId != current_user.account_id {
        return Err(WorklogError::BadInput(format!(
            "ERROR: You are not the owner of worklog with id {}",
            &instructions.worklog_id)));
    }

    client
        .delete_worklog(instructions.issue_id.clone(), instructions.worklog_id.clone())
        .await?;
    runtime
        .worklog_service()
        .remove_entry_by_worklog_id(instructions.worklog_id.as_str())?;
    Ok(instructions.worklog_id.clone())
}
