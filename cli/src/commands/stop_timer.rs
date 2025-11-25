use chrono::{DateTime, Local};
use log::{debug, info};
use worklog_core::WorklogError;

use crate::services::Services;

/// Discard the active timer without saving it
pub async fn discard_active_timer(services: &Services) -> Result<(), WorklogError> {
    debug!("Discarding active timer");

    match services.worklog.discard_active_timer().await {
        Ok(entry) => {
            info!(
                "Discarded timer for {} started at {}",
                entry.issue_key.as_deref().unwrap_or("(local)"),
                entry.started_at.format("%Y-%m-%d %H:%M")
            );
            Ok(())
        }
        Err(WorklogError::NoActiveEntry) => {
            debug!("No active timer to discard");
            Ok(())
        }
        Err(e) => {
            eprintln!("Error: Unable to discard timer: {e}");
            Err(e)
        }
    }
}

/// Stop the active timer
///
/// The `WorklogService` handles both stopping the timer locally and syncing to the issue tracker.
pub async fn stop_timer(
    services: &Services,
    stop_time: DateTime<Local>,
    comment: Option<String>,
) -> Result<(), WorklogError> {
    // Stop the timer (service handles sync automatically)
    let entry = services
        .worklog
        .stop_active_timer(stop_time, comment)
        .await?;

    let duration_seconds = entry.duration().unwrap().num_seconds();
    let hours = duration_seconds / 3600;
    let minutes = (duration_seconds % 3600) / 60;

    info!(
        "Stopped timer for issue {} with id {}, duration: {:02}:{:02}",
        entry.issue_key.as_deref().unwrap_or("(local)"),
        entry.id.as_ref().unwrap_or(&"unknown".to_string()),
        hours,
        minutes
    );

    // Report sync status
    if entry.synced_to_provider {
        if let Some(provider_id) = &entry.provider_worklog_id {
            info!("Worklog synced to Jira (ID: {provider_id})");
        }
    } else if entry.issue_key.is_some() {
        eprintln!("Warning: Worklog saved locally but not synced to Jira.");
        eprintln!("Use 'timesheet sync' to retry syncing later.");
    }

    Ok(())
}
