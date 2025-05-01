use chrono::{DateTime, Local};
use std::process::exit;
use worklog::error::WorklogError;
use worklog::{date, ApplicationRuntime};

pub(crate) fn discard_active_timer(runtime: &ApplicationRuntime) -> Result<(), WorklogError> {
    println!("Discarding active timer");
    match runtime.timer_service.discard_active_timer() {
        Ok(timer) => {
            println!(
                "Timer for {} started at {} discarded",
                timer.issue_key,
                timer.started_at.format("%Y-%m-%d %H:%M")
            );
            Ok(())
        }
        Err(WorklogError::NoActiveTimer) => {
            println!("No active timer to discard");
            Ok(())
        }
        Err(e) => {
            println!("Unable to discard timer. Cause: {e}");
            Err(e)
        }
    }
}

pub(crate) fn parse_stop_time(time_str: Option<&str>) -> DateTime<Local> {
    match time_str {
        Some(time_str) => match date::str_to_date_time(time_str) {
            Ok(datetime) => datetime,
            Err(err) => {
                eprintln!("Error: Could not parse '{time_str}' as a valid date and time: {err}");
                eprintln!("Please use one of these formats:");
                eprintln!("  - Time only (e.g., '08:00') for today at that time");
                eprintln!("  - Date only (e.g., '2023-05-26') for that date at 08:00");
                eprintln!("  - Date and time (e.g., '2023-05-26T09:00') for exact specification");
                exit(1);
            }
        },
        None => Local::now(),
    }
}

pub(crate) fn stop_timer(
    runtime: &ApplicationRuntime,
    stop_time: DateTime<Local>,
    comment: Option<String>,
) -> Result<(), WorklogError> {
    match runtime.timer_service.stop_active_timer(stop_time, comment) {
        Ok(timer) => {
            let duration_seconds = timer.duration().unwrap().num_seconds();
            let hours = duration_seconds / 3600;
            let minutes = (duration_seconds % 3600) / 60;
            println!(
                "Stopped timer for issue {} with id {:?}, duration: {:02}:{:02} ",
                timer.issue_key,
                timer.id.as_ref().unwrap(),
                hours,
                minutes
            );
            Ok(())
        }
        Err(e) => {
            println!("Unable to stop timer. Cause: {e}");
            Err(e)
        }
    }
}

pub(crate) async fn sync_timers_to_jira(runtime: &ApplicationRuntime) -> Result<(), WorklogError> {
    match runtime.timer_service.sync_timers_to_jira().await {
        Ok(timers) => {
            println!("Synced {} timers to Jira", timers.len());
            Ok(())
        }
        Err(e) => {
            println!("Unable to sync timers to Jira. Cause: {e}");
            Err(e)
        }
    }
}
