use jira::models::issue::IssueSummary;

use crate::{error::WorklogError, ApplicationRuntime};

pub(crate) async fn execute(
    runtime: &ApplicationRuntime,
) -> Result<Vec<IssueSummary>, WorklogError> {
    let jira_client = runtime.jira_client();
    let issues = jira_client
        .get_issue_summaries(&["TIME"], &[], false)
        .await?;

    Ok(issues)
}
