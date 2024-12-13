use anyhow::Result;
use chrono::{Datelike, Local, TimeZone, Weekday};
use jira::{
    models::{
        core::JiraKey,
        setting::{GlobalSettings, TimeTrackingConfiguration},
    },
    Jira,
};
use jira::{
    models::{
        core::JiraKey,
        setting::{GlobalSettings, TimeTrackingConfiguration},
    },
    Jira,
};
use log::{debug, info};

use crate::{date, error::WorklogError, storage::LocalWorklog, ApplicationRuntime};

pub struct Add {
    pub durations: Vec<String>,
    pub issue: String,
    pub started: Option<String>,
    pub comment: Option<String>,
}

pub(crate) async fn execute(
    runtime: ApplicationRuntime,
    instructions: &mut Add,
) -> Result<Vec<LocalWorklog>, WorklogError> {
    let client = runtime.jira_client();

    let cfg = client.get::<GlobalSettings>("/configuration").await?;
    let time_tracking_options = cfg.timeTrackingConfiguration;

    info!("Global Jira options: {:?}", &time_tracking_options);

    if instructions.durations.is_empty() {
        return Err(WorklogError::BadInput(
            "Need at least one duration".to_string(),
        ));
        return Err(WorklogError::BadInput(
            "Need at least one duration".to_string(),
        ));
    }

    // Ensure the issue id is always uppercase
    instructions.issue = instructions.issue.to_uppercase();

    // If there is only a single duration which does starts with a numeric
    debug!(
        "Length: {} and durations[0]: {}",
        instructions.durations.len(),
        instructions.durations[0].chars().next().unwrap()
    );

    let mut added_worklog_items: Vec<LocalWorklog> = vec![];

    if instructions.durations.len() == 1 && instructions.durations[0].chars().next().unwrap() <= '9'
    {
        // Single duration without a "day name" prefix
        // like for instance --duration 7,5h
        let result = add_single_entry(
            client,
            &time_tracking_options,
            instructions.issue.clone(),
            &instructions.durations[0],
            instructions.started.clone(),
            instructions.comment.clone(),
        )
        .await?;
        added_worklog_items.push(result);
    } else if !instructions.durations.is_empty()
        && instructions.durations[0].chars().next().unwrap() >= 'A'
    {
        // One or more durations with day name prefix, like for instance:
        // --duration mon:7,5h tue:1h wed:1d
        debug!("Handling multiple entries");
        added_worklog_items = add_multiple_entries(
            client,
            time_tracking_options,
            instructions.issue.clone(),
            instructions.durations.clone(),
            instructions.comment.clone(),
        )
        .await?;
    } else {
        return Err(WorklogError::BadInput(format!(
            "Internal error, unable to parse the durations. Did not understand: {}",
            instructions.durations[0]
        )));
    }
    // Writes the added worklog items to our local journal
    runtime
        .worklog_service()
        .add_worklog_entries(&added_worklog_items)?;

    Ok(added_worklog_items)
}

///
/// Handles list of durations specified with 3 letter abbreviations for the day name, followed by
/// ':' and the numeric duration followed by the unit ('d'=day, 'h'=hour)
/// Examples durations:
///     mon:1d tue:3,5h wed:4.5h
/// Note the decimal separator may be presented as either european format with comma (",") or US format
/// with full stop (".")
async fn add_multiple_entries(
    client: &Jira,
    time_tracking_options: TimeTrackingConfiguration,
    issue: String,
    durations: Vec<String>,
    comment: Option<String>,
) -> Result<Vec<LocalWorklog>, WorklogError> {
    // Parses the list of durations in the format XXX:nn,nnU, i.e. Mon:1,5h into Weekday, duration and unit
    let durations: Vec<(Weekday, String)> = date::parse_worklog_durations(durations);

    let mut inserted_work_logs: Vec<LocalWorklog> = vec![];

    for entry in durations {
        let weekday = entry.0;
        let duration = entry.1;

        let started = date::last_weekday(weekday);
        // Starts all entries at 08:00
        let started = Local
            .with_ymd_and_hms(started.year(), started.month(), started.day(), 8, 0, 0)
            .unwrap();

        let started = started.format("%Y-%m-%dT%H:%M").to_string();

        debug!(
            "Adding {}, {}, {}, {:?}",
            issue, &duration, started, comment
        );
        let result = add_single_entry(
            client,
            &time_tracking_options,
            issue.to_string(),
            &duration,
            Some(started),
            comment.clone(),
        )
        .await?;
        inserted_work_logs.push(result);
    }
    Ok(inserted_work_logs)
}

async fn add_single_entry(
    client: &Jira,
    time_tracking_options: &TimeTrackingConfiguration,
    issue_key: String,
    duration: &str,
    started: Option<String>,
    comment: Option<String>,
) -> Result<LocalWorklog, WorklogError> {
    debug!(
        "add_single_entry({}, {}, {:?}, {:?})",
        &issue_key, duration, started, comment
    );
    // Transforms strings like "1h", "1d", "1w" into number of seconds. Decimal point and full stop supported
    let time_spent_seconds = match date::TimeSpent::from_str(
        duration,
        time_tracking_options.workingHoursPerDay,
        time_tracking_options.workingDaysPerWeek,
    ) {
        Ok(time_spent) => time_spent.time_spent_seconds,
        Err(e) => {
            return Err(WorklogError::BadInput(
                format!(
                    "Unable to figure out the duration of your worklog entry from '{duration}', error message is: {e}"
                )
            ));
        }
    };
    debug!("time spent in seconds: {}", time_spent_seconds);

    // If a starting point was given, transform it from string to a full DateTime<Local>
    let starting_point = started
        .as_ref()
        .map(|dt| date::str_to_date_time(dt).unwrap());
    // Optionally calculates the starting point after which it is verified
    let calculated_start = date::calculate_started_time(starting_point, time_spent_seconds)?;

    let result = client
        .insert_worklog(
            issue_key.as_str(),
            calculated_start,
            time_spent_seconds,
            comment.as_deref().unwrap_or(""),
        )
        .await?;

    Ok(LocalWorklog::from_worklog(
        &result,
        JiraKey::from(issue_key),
    ))
}
