use chrono::{Days, Local};
use std::process::exit;
use worklog_core::{WorklogEntry, WorklogError};

use crate::{cli::Status, services::get_services};

/// Execute the status command using new service composition
pub async fn execute(status: Status) -> Result<(), WorklogError> {
    let services = get_services()?;

    // Get start_after date, default to 30 days ago
    let start_after = status
        .start_after
        .or_else(|| Local::now().checked_sub_days(Days::new(30)));

    eprintln!(
        "Locating local work log entries after {}",
        start_after.expect("Must specify --after ")
    );

    // Get worklogs using new unified service
    let worklogs = services
        .worklog
        .find_after(start_after.unwrap().into())
        .await?;

    // Filter by issues if specified
    let worklogs: Vec<WorklogEntry> = if let Some(keys) = status.issues {
        let key_set: std::collections::HashSet<String> = keys.into_iter().collect();
        worklogs
            .into_iter()
            .filter(|entry| {
                entry
                    .issue_key
                    .as_ref()
                    .is_some_and(|k| key_set.contains(k))
            })
            .collect()
    } else {
        worklogs
    };

    eprintln!("Found {} local worklog entries", worklogs.len());

    if worklogs.is_empty() {
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

    // Print report
    issue_and_entry_report(&worklogs);
    println!();

    // Print weekly table report - now uses WorklogEntry directly!
    // Filter out active timers
    let completed_worklogs: Vec<WorklogEntry> = worklogs
        .into_iter()
        .filter(|entry| !entry.is_active())
        .collect();

    crate::table_report_weekly::table_report_weekly(&completed_worklogs);

    // Print active timer status using new service
    match services.worklog.get_active_timer().await? {
        Some(entry) => {
            let elapsed_seconds = Local::now()
                .signed_duration_since(entry.started_at)
                .num_seconds();
            let hours = elapsed_seconds / 3600;
            let minutes = (elapsed_seconds % 3600) / 60;
            println!(
                "Active timer for {}, started at {} and current elapsed time is {:02}h {:02}m",
                entry.issue_key.as_deref().unwrap_or("(local)"),
                entry.started_at.format("%Y-%m-%d %H:%M"),
                hours,
                minutes
            );
            if let Some(comment) = entry.comment {
                println!("Timer comment: {comment}");
            } else {
                println!("No comment associated with this timer");
            }
        }
        None => {
            println!("No active timer");
        }
    }

    Ok(())
}

fn issue_and_entry_report(entries: &[WorklogEntry]) {
    println!("{:8} {:22} {:10} Comment", "Issue", "Started", "Time spent",);

    let mut sorted_entries: Vec<&WorklogEntry> = entries.iter().collect();
    sorted_entries.sort_by(|a, b| {
        a.issue_key
            .cmp(&b.issue_key)
            .then_with(|| a.started_at.cmp(&b.started_at))
    });

    for entry in &sorted_entries {
        // Skip active timers (no time_spent yet)
        if entry.is_active() {
            continue;
        }

        println!(
            "{:8} {:22} {:10} {}",
            entry.issue_key.as_deref().unwrap_or("local"),
            entry.started_at.format("%Y-%m-%d %H:%M %z"),
            crate::date_utils::seconds_to_hour_and_min(entry.time_spent_seconds.unwrap_or(0)),
            entry.comment.as_deref().unwrap_or("")
        );
    }
}
