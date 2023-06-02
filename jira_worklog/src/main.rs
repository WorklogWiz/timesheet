//! # The Jira worklog command line utility
//!
use crate::date_util::{
    calculate_started_time, date_of_last_weekday, parse_worklog_durations, str_to_date_time,
    TimeSpent,
};
use chrono::{Datelike, Local, NaiveDate, TimeZone, Weekday};
use clap::{Args, Parser, Subcommand, ValueEnum};
use env_logger::Env;
use jira_lib::{config, JiraClient, JiraIssue, TimeTrackingOptions, Worklog};
use log::{debug, info};
use std::fmt::Formatter;
use std::fs::File;
use std::{env, fmt};
use std::collections::{BTreeMap, HashMap};

mod date_util;

#[derive(Parser)]
#[command(
version = "1.0",
author = "Steinar Overbeck Cook <steinar.cook@autostoresystem.com>",
about = "Command line tool for Jira worklog mgmt"
)]
struct Opts {
    #[command(subcommand)]
    subcmd: SubCommand,

    #[arg(global = true, short, long, default_value = "info")]
    verbosity: Option<LogLevel>,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            LogLevel::Debug => write!(f, "debug"),
            LogLevel::Info => write!(f, "info"),
            LogLevel::Warn => write!(f, "warn"),
            LogLevel::Error => write!(f, "error"),
        }
    }
}

#[derive(Subcommand)]
enum SubCommand {
    /// Add worklog entries
    #[command(arg_required_else_help = true)]
    Add(Add),
    /// Delete work log entry
    Del(Del),
    /// Get status of work log entries
    Status(Status),
}

#[derive(Args)]
struct Add {
    /// Duration of work in hours (h) or days (d)
    /// If more than a single entry separate with spaces and two letter day abbreviation.
    /// --duration Mo:1,5h Tu:1d We:3,5h Fr:1d
    #[arg(short, long, num_args(1..))]
    duration: Vec<String>,
    /// Jira issue to register work on
    #[arg(short, long)]
    issue: String,
    /// work started
    #[arg(name = "started", short, long, requires = "duration")]
    started: Option<String>,
    #[arg(name = "comment", short, long)]
    comment: Option<String>,
}

#[derive(Args)]
struct Del {
    #[arg(short, long)]
    issue_id: String,
    #[arg(short = 'w', long)]
    worklog_id: String,
}

#[derive(Parser)]
struct Status {
    #[arg(short, long, num_args(1..))]
    issue: Vec<String>,
    // Consider a vector here
    #[arg(short, long)]
    after: Option<String>,
}

#[tokio::main]
async fn main() {
    let opts: Opts = Opts::parse();

    configure_logging(&opts);

    let configuration = match config::load_configuration() {
        Ok(c) => c,
        Err(e) => {
            panic!(
                "Unable to load configuration file from: {}, cause: {}",
                config::config_file_name().to_string_lossy(),
                e
            )
        }
    };

    debug!("jira_url: '{}'", configuration.jira.jira_url);
    debug!("user: '{}'", configuration.jira.user);
    debug!("token: '{}'", configuration.jira.token);

    let jira_client = match jira_lib::JiraClient::new(
        &configuration.jira.jira_url,
        &configuration.jira.user,
        &configuration.jira.token,
    ) {
        Ok(client) => client,
        Err(e) => {
            panic!("Unable to create a new http-client for Jira: {}", e)
        }
    };

    let time_tracking_options = jira_client.get_time_tracking_options().await;
    info!("Global Jira options: {:?}", &time_tracking_options);

    match opts.subcmd {
        SubCommand::Add(mut add) => {
            add.issue = add.issue.to_uppercase(); // Ensure the issue id is always uppercase

            // If there is only a single duration which does starts with a numeric
            if add.duration.len() == 1 && add.duration[0].chars().next().unwrap() <= '9' {
                println!("Adding single entry");
                add_single_entry(&jira_client, &time_tracking_options, add.issue, &add.duration[0], add.started, add.comment).await;
            } else if add.duration.len() > 1 {
                add_multiple_entries(jira_client, time_tracking_options, add.issue, add.duration, add.comment).await;
            } else {
                panic!("Internal error");
            }
        }
        SubCommand::Del(delete) => {
            let current_user = jira_client.get_current_user().await;
            let worklog_entry = jira_client
                .get_worklog(&delete.issue_id, &delete.worklog_id)
                .await;
            if worklog_entry.author.accountId != current_user.account_id {
                eprintln!(
                    "ERROR: You are not the owner of worklog with id {}",
                    &delete.worklog_id
                );
                std::process::exit(403);
            }

            match jira_client.delete_worklog(delete.issue_id, delete.worklog_id.to_owned()).await {
                Ok(_) => println!("Jira work log id {} deleted", &delete.worklog_id),
                Err(e) => println!("An error occured, worklog entry probably not deleted: {}", e),
            }
        }
        SubCommand::Status(status) => {
            // TODO: Convert started_after from String in ISO8601 form to DateTime<Local>
            let start_after = status.after.map(|s| str_to_date_time(&s).unwrap());

            let mut status_entries: Vec<Worklog> = Vec::new();
            let mut issue_information: HashMap<String, JiraIssue> = HashMap::new();

            for issue in status.issue.iter() {
                let mut entries = jira_client.get_worklogs_for_current_user(issue, start_after).await;
                status_entries.append(&mut entries);
                let issue_info = jira_client.get_issue_by_id_or_key(issue).await;
                // Allows us to lookup the issue by numeric id to augment the report
                issue_information.insert(issue_info.id.to_string(), issue_info);
            };

            issue_and_entry_report(&mut status_entries, &mut issue_information);
            summary_per_day(&mut status_entries);
        }
    }
}

fn summary_per_day(status_entries: &mut [Worklog]) {
    let mut daily_sum: BTreeMap<NaiveDate, i32> = BTreeMap::new();
    for worklog_entry in status_entries.iter() {
        let local_date = worklog_entry.started.with_timezone(&Local).date_naive();
        let _accumulated = match daily_sum.get(&local_date) {
            None => { daily_sum.insert(local_date, worklog_entry.timeSpentSeconds) }
            Some(accumulated) => {
                daily_sum.insert(local_date, accumulated + worklog_entry.timeSpentSeconds)
            }
        };
    }

    println!("{:10} {:3} {:8} ", "Date", "Day", "Duration");
    for (dt, accum) in &daily_sum {
        let hour = *accum / 3600;
        let min = *accum % 3600 / 60;
        let duration = format!("{:02}:{:02}", hour, min);
        println!("{:10} {:3} {:8}", dt, dt.weekday(), duration);
    }
}

fn issue_and_entry_report(status_entries: &mut [Worklog], issue_information: &mut HashMap<String, JiraIssue>) {
    println!("{:8} {:12} {:10} {:<7} {:28} {:-10} {:6}", "Issue", "IssueId", "Id", "Weekday", "Started", "Time spent", "In seconds");
    status_entries.sort_by(|e, other| e.issueId.cmp(&other.issueId).then_with(|| e.started.cmp(&other.started)));

    for e in status_entries.iter() {
        let issue_key: &str = match issue_information.get(&e.issueId) {
            None => "Unknown",
            Some(issue) => &issue.key
        };
        println!(
            "{:8} {:12} {:10} {:<7} {:28} {:-10} {:6}s",
            issue_key,
            e.issueId,
            e.id,
            format!("{}", e.started.weekday()),
            format!("{}", e.started.with_timezone(&Local).format("%Y-%m-%d %H:%M %z")),
            format!("{}", e.timeSpent),
            e.timeSpentSeconds
        );
    }
}

async fn add_multiple_entries(
    jira_client: JiraClient,
    time_tracking_options: TimeTrackingOptions,
    issue: String,
    durations: Vec<String>,
    comment: Option<String>,
) {
    // Parses the list of durations in the format XXXnn,nnU, i.e. Mon:1,5h into Weekday, duration and unit
    let durations: Vec<(Weekday, f32, String)> = parse_worklog_durations(durations);
    for entry in durations.into_iter() {
        let weekday = entry.0;
        let duration = entry.1;
        let unit = entry.2;

        let started = date_of_last_weekday(weekday);
        // Start all multi entries at 08:00
        let started = chrono::Local
            .with_ymd_and_hms(started.year(), started.month(), started.day(), 8, 0, 0)
            .unwrap();

        let started = started.format("%Y-%m-%dT%H:%M").to_string();
        let duration = format!("{}{}", duration, unit);
        debug!(
            "Adding {}, {}, {}, {:?}",
            issue, &duration, started, comment
        );
        add_single_entry(&jira_client, &time_tracking_options,
                         issue.to_string(), &duration, Some(started),
                         comment.clone()).await;
    }
}

async fn add_single_entry(
    jira_client: &JiraClient,
    time_tracking_options: &TimeTrackingOptions,
    issue: String,
    duration: &str,
    started: Option<String>,
    comment: Option<String>,
) {
    debug!(
        "add_single_entry({}, {}, {:?}, {:?})",
        &issue, duration, started, comment
    );
    // Transforms strings like "1h", "1d", "1w" into number of seconds. Decimal point and full stop supported
    let time_spent_seconds = match TimeSpent::from_str(
        duration,
        time_tracking_options.workingHoursPerDay,
        time_tracking_options.workingDaysPerWeek,
    ) {
        Ok(time_spent) => time_spent.time_spent_seconds,
        Err(e) => panic!(
            "Unable to figure out the duration of your worklog entry {}",
            e
        ),
    };
    debug!("time spent in seconds: {}_", time_spent_seconds);

    // If a starting point was given, transform it from string to a full DateTime<Local>
    let starting_point = started.as_ref().map(|dt| str_to_date_time(dt).unwrap());
    // Optionally calculates the starting point after which it is verified
    let calculated_start = calculate_started_time(starting_point, time_spent_seconds).unwrap();

    println!("Using these parameters as input:");
    println!("\tIssue: {}", issue.as_str());
    println!(
        "\tStarted: {}  ({})",
        calculated_start.to_rfc3339(),
        started.map_or("computed", |_| "computed from command line")
    );
    println!("\tDuration: {}s", time_spent_seconds);
    println!("\tComment: {}", comment.as_deref().unwrap_or("None"));

    let result = jira_client
        .insert_worklog(
            issue.as_str(),
            calculated_start,
            time_spent_seconds,
            comment.unwrap_or("".to_string()).as_str(),
        )
        .await;
    println!(
        "Added work log entry Id: {} Time spent: {} Time spent in seconds: {} Comment: {}",
        result.id,
        result.timeSpent,
        result.timeSpentSeconds,
        result.comment.unwrap_or("".to_string())
    )
}

fn configure_logging(opts: &Opts) {
    let mut tmp_dir = env::temp_dir();
    tmp_dir.push("jira_worklog.log");

    let target = Box::new(File::create(tmp_dir).expect("Can't create file"));

    // If nothing else was specified in RUST_LOG, use 'warn'
    env_logger::Builder::from_env(Env::default().default_filter_or(opts.verbosity.map_or(
        "warn",
        |lvl| match lvl {
            LogLevel::Debug => "debug",
            LogLevel::Info => "info",
            LogLevel::Warn => "warn",
            LogLevel::Error => "error",
        },
    )))
        .target(env_logger::Target::Pipe(target))
        .init();
}
