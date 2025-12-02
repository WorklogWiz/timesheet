//! Codes command - lists issues/codes from Jira projects

use log::info;
use worklog_core::{Issue, WorklogError};

use crate::services::Services;

pub struct CodesOptions {
    pub projects: Vec<String>,
    pub all_users: bool,
    pub all: bool,
}

/// Execute codes command - lists issues from Jira projects
pub async fn execute(
    services: &Services,
    options: CodesOptions,
) -> Result<Vec<Issue>, WorklogError> {
    let mut all_issues = Vec::new();

    // Determine the worklog filter:
    // - If --all flag is set, return ALL issues (no worklog filter)
    // - Otherwise, filter by worklog author (all_users or current user)
    let worklog_filter = if options.all {
        // No worklog filter - return ALL issues in project
        None
    } else if options.all_users {
        // Return issues where any user has logged time
        Some(true)
    } else {
        // Return issues where current user has logged time
        Some(false)
    };

    for project in &options.projects {
        info!("Searching for issues in project: {project}");

        match services
            .issue
            .search_issues_in_project(project, worklog_filter)
            .await
        {
            Ok(issues) => {
                info!("  Found {} issues", issues.len());
                all_issues.extend(issues);
            }
            Err(e) => {
                eprintln!("Error: Failed to search project {project}: {e}");
            }
        }
    }

    // Sort and deduplicate
    all_issues.sort_by(|a, b| a.key.cmp(&b.key));
    all_issues.dedup_by(|a, b| a.key == b.key);

    Ok(all_issues)
}
