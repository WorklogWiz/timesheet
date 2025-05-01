use std::process::exit;

use chrono::{Datelike, Days, Local};
use jira::models::core::IssueKey;
use log::debug;
use worklog::date;
use worklog::error::WorklogError;
use worklog::types::{LocalWorklog, Timer};
use worklog::ApplicationRuntime;

use crate::{cli::Status, get_runtime, table_report_weekly::table_report_weekly};

#[allow(clippy::unused_async)]
pub async fn execute(status: Status) -> Result<(), WorklogError> {
    let runtime = get_runtime();
    let worklog_service = runtime.worklog_service();

    let start_after = match status
        .start_after
        .map(|s| date::str_to_date_time(&s).unwrap())
    {
        None => Local::now().checked_sub_days(Days::new(30)),
        Some(date) => Some(date),
    };

    let mut jira_keys_to_report = Vec::<IssueKey>::new();
    if let Some(keys) = status.issues {
        jira_keys_to_report.extend(keys.into_iter().map(IssueKey::from));
    }

    eprintln!(
        "Locating local work log entries after {}",
        start_after.expect("Must specify --after ")
    );

    // Retrieves the data from the DBMS, which we will use to create the reports
    let worklogs = if status.all_users {
        worklog_service.find_worklogs_after(start_after.unwrap(), &jira_keys_to_report, &[])?
    } else {
        let user = runtime.user_service().find_current_user()?;
        worklog_service.find_worklogs_after(start_after.unwrap(), &jira_keys_to_report, &[user])?
    };

    eprintln!("Found {} local worklog entries", worklogs.len());
    let count_before = worklogs.iter().len();
    if count_before == 0 {
        eprintln!(
            r"ERROR: No data available in your local database for report generation.

        You should consider synchronising your relevant time codes in your local database
        with jira using this command sample command, replacing issues time-147 and time-166
        with whatever is relevant for you:

        timesheet sync -i time-147 time-166
        "
        );
        exit(2);
    }
    issue_and_entry_report(&worklogs);
    println!();
    assert_eq!(worklogs.len(), count_before);

    // Prints the report
    table_report_weekly(&worklogs);

    match get_runtime().timer_service.get_active_timer() {
        Ok(Some(timer)) => {
            let elapsed_seconds = Local::now()
                .signed_duration_since(timer.started_at)
                .num_seconds();
            let hours = elapsed_seconds / 3600;
            let minutes = (elapsed_seconds % 3600) / 60;
            println!(
                "Active timer for {}, started at {} and current elapsed time is {:02}h {:02}m",
                timer.issue_key,
                timer.started_at.format("%Y-%m-%d %H:%M"),
                hours,
                minutes
            );
        }
        Ok(None) => {
            println!("No active timer");
        }
        Err(error) => {
            eprintln!("Error when trying to find active timer: {error}");
        }
    }
    Ok(())
}

#[allow(dead_code)]
fn print_info_about_time_codes(
    runtime: &ApplicationRuntime,
    mut jira_keys_to_report: Vec<IssueKey>,
) {
    if jira_keys_to_report.is_empty() {
        jira_keys_to_report = runtime.issue_service().find_unique_keys().unwrap();
    }

    debug!(
        "Getting jira issue information for {:?}",
        &jira_keys_to_report
    );

    let result = runtime
        .issue_service()
        .get_issues_filtered_by_keys(&jira_keys_to_report)
        .expect("Unable to retrieve Jira Issue information");
    debug!("Retrieved {} entries from jira_issue table", result.len());

    println!();
    for issue in result {
        println!("{} {}", issue.issue_key, issue.summary);
    }
}

fn issue_and_entry_report(entries: &[LocalWorklog]) {
    println!(
        "{:8} {:7} {:7} {:<7} {:22} {:10} Comment",
        "Issue", "IssueId", "Id", "Weekday", "Started", "Time spent",
    );
    let mut status_entries: Vec<LocalWorklog> = entries.to_vec();
    status_entries.sort_by(|e, other| {
        e.issueId
            .cmp(&other.issueId)
            .then_with(|| e.started.cmp(&other.started))
    });

    for e in &status_entries {
        println!(
            "{:8} {:7} {:7} {:<7} {:22} {:10} {}",
            e.issue_key,
            e.issueId,
            e.id,
            format!("{}", e.started.weekday()),
            format!(
                "{}",
                e.started.with_timezone(&Local).format("%Y-%m-%d %H:%M %z")
            ),
            date::seconds_to_hour_and_min(e.timeSpentSeconds),
            e.comment.as_deref().unwrap_or("")
        );
    }
}
