use jira::models::issue::Issue;

use crate::{error::WorklogError, ApplicationRuntime};

pub(crate) async fn execute(runtime: &ApplicationRuntime) -> Result<Vec<Issue>, WorklogError> {
    let jira_client = runtime.jira_client();
    let tracking_project = &runtime.configuration().tracking_project;
    let issues = jira_client
        .get_issues_for_project(tracking_project.to_string())
        .await?;

    Ok(issues)
}
