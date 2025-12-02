//! Start timer command

use chrono::{DateTime, Local};
use log::info;
use worklog_core::WorklogError;

use crate::services::Services;

/// Start options for a timer
pub struct StartTimerOptions {
    pub issue: String,
    pub comment: Option<String>,
    pub start_time: Option<DateTime<Local>>,
}

/// Start a new timer
pub async fn start_timer(
    services: &Services,
    options: StartTimerOptions,
) -> Result<(), WorklogError> {
    let start_time = options.start_time.unwrap_or_else(Local::now);

    // Validate issue key format (basic validation)
    let issue_key = if options.issue.trim().is_empty() {
        None
    } else {
        Some(options.issue.clone())
    };

    match services
        .worklog
        .start_timer(issue_key, start_time, options.comment)
        .await
    {
        Ok(entry) => {
            info!(
                "Started timer for {} at {}",
                entry.issue_key.as_deref().unwrap_or("(local)"),
                entry.started_at.format("%Y-%m-%d %H:%M")
            );
            if let Some(comment) = &entry.comment {
                info!("Comment: {comment}");
            }
            Ok(())
        }
        Err(WorklogError::ActiveEntryExists) => {
            eprintln!("Error: There is already an active timer running.");
            eprintln!("Stop or discard the current timer before starting a new one.");
            Err(WorklogError::ActiveEntryExists)
        }
        Err(e) => {
            eprintln!("Error: Unable to start timer: {e}");
            Err(e)
        }
    }
}
