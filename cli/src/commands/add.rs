//! Add command - adds worklogs directly to Jira (and optionally saves locally)

use chrono::{DateTime, Datelike, Local, Weekday};
use log::info;
use worklog_core::{WorklogEntry, WorklogError};

use crate::date_utils::DurationEntry;
use crate::services::Services;

pub struct AddOptions {
    pub durations: Vec<DurationEntry>,
    pub issue_key: String,
    pub started: Option<DateTime<Local>>,
    pub comment: Option<String>,
}

/// Execute add command - adds worklogs directly to Jira
pub async fn execute(
    services: &Services,
    options: AddOptions,
) -> Result<Vec<WorklogEntry>, WorklogError> {
    if options.durations.is_empty() {
        return Err(WorklogError::ValidationError(
            "No durations specified".to_string(),
        ));
    }

    // Parse durations and create worklog entries
    let entries = parse_durations(&options);

    let mut added_entries = Vec::new();

    for entry in entries {
        // Add and sync using the service
        let saved_entry = services.worklog.add_and_sync(&entry).await?;

        info!(
            "Added worklog: {} - {} - {}",
            options.issue_key,
            crate::date_utils::seconds_to_hour_and_min(entry.time_spent_seconds.unwrap_or(0)),
            entry.comment.as_deref().unwrap_or("")
        );

        added_entries.push(saved_entry);
    }

    Ok(added_entries)
}

fn parse_durations(options: &AddOptions) -> Vec<WorklogEntry> {
    let mut entries = Vec::new();

    // Determine base start time
    let base_start = options.started.unwrap_or_else(Local::now);

    for duration_entry in &options.durations {
        // Calculate start time based on weekday (if specified)
        let start_time = if let Some(target_weekday) = duration_entry.weekday {
            // Adjust to the specified weekday
            adjust_to_weekday(base_start, target_weekday)
        } else {
            base_start
        };

        // Create worklog entry (already completed)
        let mut entry =
            WorklogEntry::start_now(Some(options.issue_key.clone()), options.comment.clone());
        entry.started_at = start_time;
        entry.stopped_at = Some(start_time + chrono::Duration::seconds(duration_entry.seconds));
        #[allow(clippy::cast_possible_truncation)]
        {
            entry.time_spent_seconds = Some(duration_entry.seconds as i32);
        }

        entries.push(entry);
    }

    entries
}

fn adjust_to_weekday(base: chrono::DateTime<Local>, target: Weekday) -> chrono::DateTime<Local> {
    let current_weekday = base.weekday();
    #[allow(clippy::cast_possible_wrap)]
    let days_diff =
        (target.num_days_from_monday() as i32) - (current_weekday.num_days_from_monday() as i32);

    if days_diff >= 0 {
        base - chrono::Duration::days(i64::from(7 - days_diff))
    } else {
        base - chrono::Duration::days(i64::from(-days_diff))
    }
}
