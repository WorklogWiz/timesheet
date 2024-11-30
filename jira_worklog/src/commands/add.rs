use std::process::exit;

use chrono::{Datelike, Local, TimeZone, Weekday};
use common::date;
use jira_lib::{
    models::{core::JiraKey, setting::TimeTrackingConfiguration},
    Jira,
};
use local_worklog::LocalWorklog;
use log::{debug, info};
use reqwest::StatusCode;

use crate::{cli::Add, get_jira_client, get_runtime};

pub async fn execute(add: &mut Add) {
    let runtime = get_runtime();
    let jira_client = get_jira_client(runtime.get_application_configuration());

    let time_tracking_options = match jira_client.get_time_tracking_options().await {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Failed to create the Jira client. Http status code {e}");
            exit(4);
        }
    };

    info!("Global Jira options: {:?}", &time_tracking_options);

    if add.durations.is_empty() {
        eprintln!("Must specify a duration with --duration");
        exit(4);
    }

    add.issue = add.issue.to_uppercase(); // Ensure the issue id is always uppercase

    // If there is only a single duration which does starts with a numeric
    debug!(
        "Length: {} and durations[0]: {}",
        add.durations.len(),
        add.durations[0].chars().next().unwrap()
    );

    let mut added_worklog_items: Vec<LocalWorklog> = vec![];

    if add.durations.len() == 1 && add.durations[0].chars().next().unwrap() <= '9' {
        // Single duration without a "day name" prefix
        // like for instance --duration 7,5h
        let result = add_single_entry(
            &jira_client,
            &time_tracking_options,
            add.issue.clone(),
            &add.durations[0],
            add.started.clone(),
            add.comment.clone(),
        )
        .await;
        added_worklog_items.push(result);
    } else if !add.durations.is_empty() && add.durations[0].chars().next().unwrap() >= 'A' {
        // One or more durations with day name prefix, like for instance:
        // --duration mon:7,5h tue:1h wed:1d
        debug!("Handling multiple entries");
        added_worklog_items = add_multiple_entries(
            jira_client,
            time_tracking_options,
            add.issue.clone(),
            add.durations.clone(),
            add.comment.clone(),
        )
        .await;
    } else {
        eprintln!(
            "Internal error, unable to parse the durations. Did not understand: {}",
            add.durations[0]
        );
        exit(4);
    }
    // Writes the added worklog items to our local journal
    if let Err(e) = runtime
        .get_local_worklog_service()
        .add_worklog_entries(added_worklog_items)
    {
        eprintln!("Failed to add worklog entries to local data store: {e}");
        exit(4);
    }
}

///
/// Handles list of durations specified with 3 letter abbreviations for the day name, followed by
/// ':' and the numeric duration followed by the unit ('d'=day, 'h'=hour)
/// Examples durations:
///     mon:1d tue:3,5h wed:4.5h
/// Note the decimal separator may be presented as either european format with comma (",") or US format
/// with full stop (".")
async fn add_multiple_entries(
    jira_client: Jira,
    time_tracking_options: TimeTrackingConfiguration,
    issue: String,
    durations: Vec<String>,
    comment: Option<String>,
) -> Vec<LocalWorklog> {
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
            &jira_client,
            &time_tracking_options,
            issue.to_string(),
            &duration,
            Some(started),
            comment.clone(),
        )
        .await;
        inserted_work_logs.push(result);
    }
    inserted_work_logs
}

async fn add_single_entry(
    jira_client: &Jira,
    time_tracking_options: &TimeTrackingConfiguration,
    issue_key: String,
    duration: &str,
    started: Option<String>,
    comment: Option<String>,
) -> LocalWorklog {
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
            eprintln!("Unable to figure out the duration of your worklog entry from '{duration}', error message is: {e}");
            exit(4);
        }
    };
    debug!("time spent in seconds: {}", time_spent_seconds);

    // If a starting point was given, transform it from string to a full DateTime<Local>
    let starting_point = started
        .as_ref()
        .map(|dt| date::str_to_date_time(dt).unwrap());
    // Optionally calculates the starting point after which it is verified
    let calculated_start = date::calculate_started_time(starting_point, time_spent_seconds)
        .unwrap_or_else(|err: date::Error| {
            eprintln!("{err}");
            exit(4);
        });

    println!("Using these parameters as input:");
    println!("\tIssue: {}", issue_key.as_str());
    println!(
        "\tStarted: {}  ({})",
        calculated_start.to_rfc3339(),
        started.map_or("computed", |_| "computed from command line")
    );
    println!("\tDuration: {time_spent_seconds}s");
    println!("\tComment: {}", comment.as_deref().unwrap_or("None"));

    let result = match jira_client
        .insert_worklog(
            issue_key.as_str(),
            calculated_start,
            time_spent_seconds,
            comment.as_deref().unwrap_or(""),
        )
        .await
    {
        Ok(result) => result,
        Err(e) => match e {
            StatusCode::NOT_FOUND => {
                eprintln!("WARNING: Issue {issue_key} not found");
                exit(4);
            }
            other => {
                eprintln!("ERROR: Unable to insert worklog entry for issue {issue_key}, http error code {other}");
                exit(4);
            }
        },
    };

    println!(
        "Added work log entry Id: {} Time spent: {} Time spent in seconds: {} Comment: {}",
        &result.id,
        &result.timeSpent,
        &result.timeSpentSeconds,
        &result.comment.as_deref().unwrap_or("")
    );
    println!(
        "To delete entry: jira_worklog del -i {} -w {}",
        issue_key, &result.id
    );

    LocalWorklog::from_worklog(&result, JiraKey::from(issue_key))
}
