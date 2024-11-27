//! The Jira worklog command line utility
//!
use std::fmt::Formatter;
use std::fs::File;
use std::process::exit;
use std::{env, fmt};

use chrono::{DateTime, Datelike, Days, Local, TimeZone, Weekday};
use clap::{Args, Parser, Subcommand, ValueEnum};
use env_logger::Env;
use log::{debug, info};
use reqwest::StatusCode;

use common::{config, date, WorklogError};
use jira_lib::{JiraClient, JiraKey, TimeTrackingConfiguration};
use local_worklog::{LocalWorklog, LocalWorklogService};
use worklog_lib::ApplicationRuntime;

mod table_report_weekly;

#[derive(Parser)]
/// Jira worklog utility - add, delete and list jira worklog entries
///
/// Dates should be specified in the ISO8601 format without a time zone. Local timezone is
/// always assumed. I.e. `2023-06-01`.
///
/// Duration is specified in units of hours, days or weeks, using the abbreviations 'h','d', and 'w'
/// respectively.
/// Duration may use either the period or the comma to separate the fractional part of a number.
///
/// 7,5h and 7.5h both indicate 7 hours and 30 minutes, and so does 7h30m
/// 7:30 specifies 7 hours and 30 minutes.
///
///
#[command(author, about)] // Read from Cargo.toml
struct Opts {
    #[command(subcommand)]
    subcmd: SubCommand,

    #[arg(global = true, short, long, global = true)]
    verbosity: Option<LogLevel>,
}
const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Subcommand)]
enum SubCommand {
    /// Add worklog entries
    #[command(arg_required_else_help = true)]
    Add(Add),
    /// Delete work log entry
    #[command(arg_required_else_help = true)]
    Del(Del),
    /// Get status of work log entries
    Status(Status),
    #[command(arg_required_else_help = true)]
    Config(Configuration),
    /// Lists all time codes
    Codes,
    /// Synchronize local data store with remote Jira work logs
    Sync(Synchronisation),
}

#[derive(Args)]
struct Add {
    /// Duration of work in hours (h) or days (d)
    /// If more than a single entry separate with spaces and three letter abbreviation of
    /// weekday name:
    ///     --durations Mon:1,5h Tue:1d Wed:3,5h Fri:1d
    #[arg(short, long, num_args(1..))]
    durations: Vec<String>,
    /// Jira issue to register work on
    #[arg(short, long, required = true)]
    issue: String,
    /// work started
    #[arg(name = "started", short, long, requires = "durations")]
    started: Option<String>,
    #[arg(name = "comment", short, long)]
    comment: Option<String>,
}

#[derive(Args)]
struct Del {
    #[arg(short, long, required = true)]
    issue_id: String,
    #[arg(short = 'w', long, required = true)]
    worklog_id: String,
}

#[derive(Parser)]
struct Status {
    /// Issues to be reported on. If no issues are specified.
    /// The unique Jira keys found in the local journal of entries is used.
    /// You can specify a list of issue keys: -i time-147 time-148
    #[arg(short, long, num_args(1..), required = false)]
    issues: Option<Vec<String>>,
    #[arg(short, long)]
    /// Retrieves all entries after the given date
    after: Option<String>,
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

/// Create, modify or list the configuration file.
/// The configuration file will be automagically created if you use `--token`, `--user` or `--jira_url`
#[derive(Parser)]
struct Configuration {
    /// The Jira security API token obtained from your Manage Account -> Security
    #[arg(short, long)]
    token: Option<String>,
    /// Your email address, i.e. steinar.cook@autostoresystem.com
    #[arg(short, long)]
    user: Option<String>,
    /// Lists the current configuration (if it exists) and exit
    #[arg(short, long)]
    list: bool,
    /// The URL of Jira, don't change this unless you know what you are doing
    #[arg(
        short,
        long,
        default_value = "https://autostore.atlassian.net/rest/api/latest"
    )]
    jira_url: Option<String>,
    #[arg(long, default_value_t = false)]
    remove: bool,
}

#[derive(Parser)]
struct Synchronisation {
    #[arg(name = "started", short, long)]
    /// Default is to sync for the current month, but you may specify an ISO8601 date from which
    /// data should be synchronised
    started: Option<String>,
    #[arg(
        name = "issues",
        short,
        long,
        long_help = "Limit synchronisation to these issues"
    )]
    issues: Vec<String>,
}

#[tokio::main]
async fn main() {
    println!("Version: {VERSION}");

    let opts: Opts = Opts::parse();

    configure_logging(&opts); // Handles the -v option

    if let Ok(entry_count) = worklog_lib::migrate_csv_journal_to_local_worklog_dbms(None).await {
        debug!(
            "Migrated {} entries from CVS Journal to local work log DBMS",
            entry_count
        );
    } else {
        info!("No local CSV Journal entries migrated");
    }

    match opts.subcmd {
        SubCommand::Add(mut add) => {
            add_subcommand(&mut add).await;
        }

        SubCommand::Del(delete) => {
            delete_subcommand(&delete).await;
        }

        SubCommand::Status(status) => {
            status_subcommand(status).await;
        }

        SubCommand::Config(config) => match config {
            // List current configuration
            Configuration {
                list: true,
                remove: false,
                ..
            } => {
                list_config_and_exit();
            }
            // Add new values to the configuration
            Configuration {
                user,
                token,
                jira_url,
                list: false,
                remove: false,
            } => {
                let mut app_config = match config::load_or_create() {
                    Ok(ac) => ac,
                    Err(e) => {
                        eprintln!(
                            "ERROR: Unable to load or create configuration file {}, reason:{}",
                            config::configuration_file_name().to_string_lossy(),
                            e
                        );
                        exit(4);
                    }
                };
                if let Some(user) = user {
                    app_config.jira.user = user.to_string();
                }
                if let Some(token) = token {
                    app_config.jira.token = token.to_string();
                }
                if let Some(jira_url) = jira_url {
                    app_config.jira.jira_url = jira_url.to_string();
                }
                config::save(&app_config).expect("Unable to save the application config");
                println!(
                    "Configuration saved to {}",
                    config::configuration_file_name().to_string_lossy()
                );
                exit(0);
            }
            Configuration { remove: true, .. } => match config::remove() {
                Ok(()) => {
                    println!(
                        "Configuration file {} removed",
                        config::configuration_file_name().to_string_lossy()
                    );
                }
                Err(e) => {
                    println!(
                        "ERROR:Unable to remove configuration file {} : {}",
                        config::configuration_file_name().to_string_lossy(),
                        e
                    );
                }
            },
        }, // end Config
        SubCommand::Codes => {
            let runtime = get_runtime();
            let jira_client = get_jira_client(runtime.get_application_configuration());
            let issues = jira_client
                .get_issues_for_single_project("TIME".to_string())
                .await;
            for issue in issues {
                println!("{} {}", issue.key, issue.fields.summary);
            }
        }
        SubCommand::Sync(synchronisation) => {
            let _result = sync_subcommand(synchronisation).await;
        }
    }
}

async fn sync_subcommand(sync: Synchronisation) -> anyhow::Result<()> {
    let runtime = ApplicationRuntime::new_production()?;
    let start_after = sync.started.map(|s| date::str_to_date_time(&s).unwrap());

    let mut issue_keys_to_sync = sync.issues.clone();
    if issue_keys_to_sync.is_empty() {
        issue_keys_to_sync = runtime.get_local_worklog_service().find_unique_keys()?;
    }
    if issue_keys_to_sync.is_empty() {
        eprintln!(
            "No issue keys to synchronise supplied on commandline or found in the local dbms"
        );
        exit(4);
    }

    println!("Synchronising work logs for these issues:");
    for issue in &issue_keys_to_sync {
        println!("\t{issue}");
    }
    debug!(
        "Synchronising with Jira for these issues {:?}",
        &issue_keys_to_sync
    );

    // Retrieve the work logs for each issue key specified on the command line
    for issue_key in &issue_keys_to_sync {
        let worklogs = runtime
            .get_jira_client()
            .get_worklogs_for_current_user(issue_key, start_after)
            .await
            .map_err(|e| WorklogError::JiraResponse {
                msg: format!("unable to get worklogs for current user {e}").to_string(),
                reason: e.to_string(),
            })?;
        // ... and insert them into our local data store
        println!(
            "Synchronising {} entries for time code {}",
            worklogs.len(),
            &issue_key
        );
        for worklog in worklogs {
            debug!("Removing and adding {:?}", &worklog);

            // Delete the existing one if it exists
            if let Err(e) = runtime.get_local_worklog_service().remove_entry(&worklog) {
                debug!("Unable to remove {:?}: {}", &worklog, e);
            }

            debug!("Adding {} {:?}", &issue_key, &worklog);

            let local_worklog =
                LocalWorklog::from_worklog(&worklog, JiraKey::from(issue_key.clone()));
            if let Err(err) = runtime
                .get_local_worklog_service()
                .add_entry(&local_worklog)
            {
                eprintln!(
                    "Insert into database failed for {:?}, cause: {:?}",
                    &local_worklog, err
                );
                exit(4);
            }
        }
    }
    let keys: Vec<JiraKey> = issue_keys_to_sync
        .iter()
        .map(|s| JiraKey::from(s.as_str()))
        .collect();
    let issue_info = runtime.sync_jira_issue_information(&keys).await?;
    println!();
    for issue in issue_info {
        println!("{:12} {}", issue.key, issue.fields.summary);
    }

    Ok(())
}

async fn delete_subcommand(delete: &Del) {
    let runtime = get_runtime();
    let jira_client = get_jira_client(runtime.get_application_configuration());

    let current_user = jira_client.get_current_user().await;
    let worklog_entry = match jira_client
        .get_worklog(&delete.issue_id, &delete.worklog_id)
        .await
    {
        Ok(result) => result,
        Err(e) => match e {
            StatusCode::NOT_FOUND => {
                eprintln!(
                    "Worklog {} for issue '{}' not found",
                    &delete.worklog_id, &delete.issue_id
                );
                exit(4);
            }
            other => {
                eprintln!("ERROR: unknown http status code: {other}");
                exit(16)
            }
        },
    };

    if worklog_entry.author.accountId != current_user.account_id {
        eprintln!(
            "ERROR: You are not the owner of worklog with id {}",
            &delete.worklog_id
        );
        exit(403);
    }

    match jira_client
        .delete_worklog(delete.issue_id.clone(), delete.worklog_id.clone())
        .await
    {
        Ok(()) => println!("Jira work log id {} deleted from Jira", &delete.worklog_id),
        Err(e) => {
            println!("An error occurred, worklog entry probably not deleted: {e}");
            exit(4);
        }
    }
    match runtime
        .get_local_worklog_service()
        .remove_entry_by_worklog_id(delete.worklog_id.as_str())
    {
        Ok(()) => {
            println!("Removed entry {} from local worklog", delete.worklog_id);
        }
        Err(err) => {
            panic!(
                "Deletion from local worklog failed for worklog.id = '{}' : {err}",
                delete.worklog_id.as_str()
            );
        }
    }
}
#[allow(clippy::unused_async)]
async fn status_subcommand(status: Status) {
    let worklog_service = LocalWorklogService::new(&config::local_worklog_dbms_file_name())
        .expect("Unable to create the local worklog servicer ");

    let start_after = match status.after.map(|s| date::str_to_date_time(&s).unwrap()) {
        None => Local::now().checked_sub_days(Days::new(30)),
        Some(date) => Some(date),
    };

    let mut jira_keys_to_report = Vec::<JiraKey>::new();
    if let Some(keys) = status.issues {
        jira_keys_to_report.extend(keys.into_iter().map(JiraKey::from));
    }

    eprintln!(
        "Locating local worklog entries after {}",
        start_after.expect("Must specify --after ")
    );
    let worklogs =
        match worklog_service.find_worklogs_after(start_after.unwrap(), &jira_keys_to_report) {
            Ok(worklogs) => worklogs,
            Err(e) => {
                eprintln!("Unable to retrieve worklogs from local work log database {e}");
                exit(4);
            }
        };

    eprintln!("Found {} local worklog entries", worklogs.len());
    let count_before = worklogs.iter().len();
    issue_and_entry_report(&worklogs);
    println!();
    assert_eq!(worklogs.len(), count_before);

    // Prints the report
    table_report_weekly::table_report_weekly(&worklogs);

    print_info_about_time_codes(&worklog_service, jira_keys_to_report);
}

fn print_info_about_time_codes(
    worklog_service: &LocalWorklogService,
    mut jira_keys_to_report: Vec<JiraKey>,
) {
    if jira_keys_to_report.is_empty() {
        jira_keys_to_report = worklog_service
            .find_unique_keys()
            .unwrap_or_default()
            .iter()
            .map(|k| JiraKey::from(k.as_str()))
            .collect();
    }

    debug!(
        "Getting jira issue information for {:?}",
        &jira_keys_to_report
    );

    let result = worklog_service
        .get_jira_issues_filtered_by_keys(&jira_keys_to_report)
        .expect("Unable to retrieve Jira Issue information");
    debug!("Retrieved {} entries from jira_issue table", result.len());

    println!();
    for issue in result {
        println!("{} {}", issue.issue_key, issue.summary);
    }
}

#[allow(dead_code)]
async fn fetch_worklog_entries_from_jira_for_key(
    jira_client: &JiraClient,
    start_after: Option<DateTime<Local>>,
    issue_key: &String,
) -> Vec<LocalWorklog> {
    match jira_client
        .get_worklogs_for_current_user(issue_key, start_after)
        .await
    {
        Ok(result) => {
            let mut work_logs = vec![];
            for worklog in result {
                let local_worklog =
                    LocalWorklog::from_worklog(&worklog, JiraKey::from(issue_key.as_str()));
                work_logs.push(local_worklog);
            }
            work_logs
        }
        Err(e) => match e {
            StatusCode::NOT_FOUND => {
                eprintln!("Issue {issue_key} not found");
                exit(4);
            }
            other => {
                eprintln!("ERROR: Unknown http status code {other} for issue {issue_key}");
                exit(16);
            }
        },
    }
}

async fn add_subcommand(add: &mut Add) {
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

/// Creates the `JiraClient` instance based on the supplied parameters.
fn get_jira_client(app_config: &config::AppConfiguration) -> JiraClient {
    match JiraClient::new(
        &app_config.jira.jira_url,
        &app_config.jira.user,
        &app_config.jira.token,
    ) {
        Ok(client) => client,
        Err(e) => {
            eprintln!("ERROR: Unable to create a new http-client for Jira: {e}");
            exit(8);
        }
    }
}

/// Retrieves the application configuration file
fn get_runtime() -> ApplicationRuntime {
    match ApplicationRuntime::new_production() {
        Ok(runtime) => runtime,
        Err(err) => {
            println!("Unable to load application runtime configuration {err}");
            println!("Create it with: jira_worklog config --user <EMAIL> --token <JIRA_TOKEN>");
            println!("See 'config' subcommand for more details");
            exit(4);
        }
    }
}

fn list_config_and_exit() {
    println!(
        "Configuration file {}:\n",
        config::configuration_file_name().to_string_lossy()
    );

    match config::load() {
        Ok(config) => {
            let toml_as_string = config::application_config_to_string(&config).unwrap();
            println!("{toml_as_string}");
        }
        Err(_) => {
            println!("Config file does not exist or is empty. Use --token and --user to create it");
        }
    }
    exit(0);
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
            date::seconds_to_hour_and_min(&e.timeSpentSeconds),
            e.comment.as_deref().unwrap_or("")
        );
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
    jira_client: JiraClient,
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
    jira_client: &JiraClient,
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

fn configure_logging(opts: &Opts) {
    let mut tmp_dir = env::temp_dir();
    tmp_dir.push("jira_worklog.log");

    if opts.verbosity.is_some() {
        println!("Logging to {}", &tmp_dir.to_string_lossy());
    }

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
    debug!("Logging started");
}

#[allow(dead_code)]
pub fn print_sum_per_week(sum_per_week: &mut i32, week: u32) {
    println!("{:-<23}", "");
    println!(
        "ISO week {}, sum: {} ",
        week,
        date::seconds_to_hour_and_min(sum_per_week)
    );
    println!("{:=<23}", "");
    println!();
}
