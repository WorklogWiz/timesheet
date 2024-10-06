//! # The Jira worklog command line utility
//!
use jira_lib::date_util::{
    calculate_started_time, date_of_last_weekday, DateTimeError, parse_worklog_durations,
    str_to_date_time, TimeSpent,
};
use chrono::{Datelike, Local, NaiveDate, TimeZone, Weekday};
use clap::{Args, Parser, Subcommand, ValueEnum};
use env_logger::Env;
use jira_lib::config::{
    ApplicationConfig, config_file_name, load_or_create_configuration, remove_configuration,
    save_configuration,
};
use jira_lib::{config, date_util, JiraClient, JiraIssue, JiraKey, journal, TimeTrackingConfiguration, Worklog};
use jira_lib::journal::{add_worklog_entries_to_journal, JournalEntry};

use log::{debug, info};
use reqwest::StatusCode;
use std::collections::{BTreeMap, HashMap};
use std::fmt::Formatter;
use std::fs::File;
use std::process::exit;
use std::{env, fmt};
use std::path::PathBuf;

mod table_report;


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
/// 7,5h or 7.5h both indicate 7 hours and 30 minutes
/// 7:30 specifies 7 hours and 30 minutes
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
/// The configuration file will be automagically created if you use --token, --user or --jira_url
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

#[tokio::main]
async fn main() {
    println!("Version: {}", VERSION);

    let opts: Opts = Opts::parse();
    configure_logging(&opts); // Handles the -v option

    match opts.subcmd {
        SubCommand::Add(mut add) => {
            let app_config = get_app_config();
            let jira_client = get_jira_client(&app_config);

            let time_tracking_options = match jira_client.get_time_tracking_options().await {
                Ok(t) => t,
                Err(e) => {
                    eprintln!("Failed to create the Jira client. Http status code {}", e);
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

            let mut added_worklog_items: Vec<JournalEntry> = vec![];

            if add.durations.len() == 1 && add.durations[0].chars().next().unwrap() <= '9' {
                // Single duration without a "day name" prefix
                // like for instance --duration 7,5h
                println!("Adding single entry");
                let result = add_single_entry(
                    &jira_client,
                    &time_tracking_options,
                    add.issue,
                    &add.durations[0],
                    add.started,
                    add.comment,
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
                    add.issue,
                    add.durations,
                    add.comment,
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
            add_worklog_entries_to_journal(added_worklog_items);
        }

        SubCommand::Del(delete) => {
            let app_config = get_app_config();
            let jira_client = get_jira_client(&app_config);

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
                        eprintln!("ERROR: unknown http status code: {}", other);
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
                .delete_worklog(delete.issue_id, delete.worklog_id.to_owned())
                .await
            {
                Ok(_) => println!("Jira work log id {} deleted", &delete.worklog_id),
                Err(e) => {
                    println!(
                        "An error occurred, worklog entry probably not deleted: {}",
                        e
                    );
                    exit(4);
                }
            }
            journal::remove_entry_from_journal(&PathBuf::from(app_config.application_data.journal_data_file_name), delete.worklog_id.as_str());
            println!("Removed entry {} from local journal", delete.worklog_id);
        }

        SubCommand::Status(status) => {
            let app_config = get_app_config();

            let jira_client = get_jira_client(&app_config);
            let start_after = status.after.map(|s| str_to_date_time(&s).unwrap());

            let mut worklog_entries: Vec<Worklog> = Vec::new();
            let mut issue_information: HashMap<String, JiraIssue> = HashMap::new();

            let keys = if status.issues.is_none() {
                journal::find_unique_keys(&PathBuf::from(app_config.application_data.journal_data_file_name))
            } else {
                status.issues.unwrap()
            };
            if keys.is_empty() {
                eprintln!("No issues provided on command line and no issues found in local journal");
                eprintln!("You want to use the -i option and specify issues");
                exit(4);
            }
            eprintln!("Retrieving data for time codes: {}", &keys.join(", "));

            for issue in keys.iter() {
                let mut entries = match jira_client
                    .get_worklogs_for_current_user(issue, start_after)
                    .await
                {
                    Ok(result) => result,
                    Err(e) => match e {
                        StatusCode::NOT_FOUND => {
                            eprintln!("Issue {} not found", issue);
                            exit(4);
                        }
                        other => {
                            eprintln!(
                                "ERROR: Unknown http status code {} for issue {}",
                                other, issue
                            );
                            exit(16);
                        }
                    },
                };
                worklog_entries.append(&mut entries);
                let issue_info = match jira_client.get_issue_by_id_or_key(issue).await {
                    Ok(r) => r,
                    Err(e) => match e {
                        StatusCode::NOT_FOUND => {
                            eprintln!("Issue {} not found", issue);
                            exit(4);
                        }
                        other => {
                            eprintln!(
                                "ERROR: Unknown http status code {} for issue {}",
                                other, issue
                            );
                            exit(4);
                        }
                    },
                };
                // Allows us to look up the issue by numeric id to augment the report
                issue_information.insert(issue_info.id.to_string(), issue_info);
            }

            issue_and_entry_report(&mut worklog_entries, &mut issue_information);
            println!();

            let issue_keys_by_command_line_order = keys
                .iter()
                .map(|k| JiraKey(k.to_owned()))
                .collect();
            table_report::table_report(
                &mut worklog_entries,
                &issue_keys_by_command_line_order,
                &issue_information,
            );
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
                let mut app_config = match load_or_create_configuration() {
                    Ok(ac) => ac,
                    Err(e) => {
                        eprintln!(
                            "ERROR: Unable to load or create configuration file {}, reason:{}",
                            config_file_name().to_string_lossy(),
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
                save_configuration(app_config);
                println!(
                    "Configuration saved to {}",
                    config_file_name().to_string_lossy()
                );
                exit(0);
            }
            Configuration { remove: true, .. } => match remove_configuration() {
                Ok(_) => {
                    println!(
                        "Configuration file {} removed",
                        config_file_name().to_string_lossy()
                    )
                }
                Err(e) => {
                    println!(
                        "ERROR:Unable to remove configuration file {} : {}",
                        config_file_name().to_string_lossy(),
                        e
                    )
                }
            },
        }, // end Config
        SubCommand::Codes => {
            let app_config = get_app_config();
            let jira_client = get_jira_client(&app_config);
            let issues = jira_client.get_issues_for_single_project("TIME".to_string()).await;
            for issue in issues {
                println!("{} {}", issue.key, issue.fields.summary);
            }
        }
    }
}

fn get_jira_client(app_config: &ApplicationConfig) -> JiraClient {

    match JiraClient::new(
        &app_config.jira.jira_url,
        &app_config.jira.user,
        &app_config.jira.token,
    ) {
        Ok(client) => client,
        Err(e) => {
            eprintln!("ERROR: Unable to create a new http-client for Jira: {}", e);
            exit(8);
        }
    }
}

fn get_app_config() -> ApplicationConfig {
    let app_config = match config::load_configuration() {
        Ok(c) => c,
        Err(_) => {
            println!(
                "Config file {} not found.",
                config_file_name().to_string_lossy()
            );
            println!("Create it with: jira_worklog config --user <EMAIL> --token <JIRA_TOKEN>");
            println!("See 'config' subcommand for more details");
            exit(4);
        }
    };

    debug!("jira_url: '{}'", app_config.jira.jira_url);
    debug!("user: '{}'", app_config.jira.user);
    debug!("token: '{}'", app_config.jira.token);
    app_config
}

fn list_config_and_exit() {
    println!(
        "Configuration file {}:\n",
        config_file_name().to_string_lossy()
    );
    match config::load_configuration() {
        Ok(config) => {
            let toml_as_string = config::application_config_to_string(&config);
            println!("{}", toml_as_string);
        }
        Err(_) => {
            println!("Config file does not exist or is empty. Use --token and --user to create it")
        }
    }
    exit(0);
}


#[allow(dead_code, deprecated)]
fn old_weekly_summary_report(worklog_entries: &mut [Worklog]) {
    // Accumulates the total amount of hours per day
    let mut daily_sum: BTreeMap<NaiveDate, i32> = BTreeMap::new();
    for worklog_entry in worklog_entries.iter() {
        let local_date = worklog_entry.started.with_timezone(&Local).date_naive();
        let _accumulated = match daily_sum.get(&local_date) {
            None => daily_sum.insert(local_date, worklog_entry.timeSpentSeconds),
            Some(accumulated) => {
                daily_sum.insert(local_date, accumulated + worklog_entry.timeSpentSeconds)
            }
        };
    }

    let mut sum_per_week = 0;
    let mut current_week = 0;
    let mut sum_per_month = 0;
    let mut current_month = 0;
    let mut monthly_totals: BTreeMap<u32, i32> = BTreeMap::new();

    println!("CW {:10} {:3} {:8} ", "Date", "Day", "Duration");
    for (dt, accum_per_day) in &daily_sum {
        if current_week == 0 {
            current_week = dt.iso_week().week();
        }
        if current_month == 0 {
            current_month = dt.month();
        }

        if date_util::is_new_week(current_week, dt) {
            print_sum_per_week(&mut sum_per_week, dt.iso_week().week() - 1);
            current_week = dt.iso_week().week();
            sum_per_week = 0;
        }
        // Excludes current month
        if dt.month() > current_month {
            monthly_totals.insert(current_month, sum_per_month);
            current_month = dt.month();
            sum_per_month = 0;
        }
        let duration_this_day = date_util::seconds_to_hour_and_min(accum_per_day);
        println!(
            "{:2} {:10} {:3} {:8}",
            dt.iso_week().week(),
            dt,
            dt.weekday(),
            duration_this_day
        );
        sum_per_week += accum_per_day;
        sum_per_month += accum_per_day;
    }
    print_sum_per_week(&mut sum_per_week, Local::now().iso_week().week());

    println!();
    for (month, total) in monthly_totals {
        println!(
            "{:9} {}",
            date_util::month_name(month).name(),
            date_util::seconds_to_hour_and_min(&total)
        );
    }
}

fn issue_and_entry_report(
    status_entries: &mut [Worklog],
    issue_information: &mut HashMap<String, JiraIssue>,
) {
    println!(
        "{:8} {:7} {:7} {:<7} {:22} {:10} Comment",
        "Issue", "IssueId", "Id", "Weekday", "Started", "Time spent",
    );
    status_entries.sort_by(|e, other| {
        e.issueId
            .cmp(&other.issueId)
            .then_with(|| e.started.cmp(&other.started))
    });

    for e in status_entries.iter() {
        let issue_key: &str = match issue_information.get(&e.issueId) {
            None => "Unknown",
            Some(issue) => issue.key.value(),
        };
        println!(
            "{:8} {:7} {:7} {:<7} {:22} {:10} {}",
            issue_key,
            e.issueId,
            e.id,
            format!("{}", e.started.weekday()),
            format!(
                "{}",
                e.started.with_timezone(&Local).format("%Y-%m-%d %H:%M %z")
            ),
            date_util::seconds_to_hour_and_min(&e.timeSpentSeconds),
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
) -> Vec<JournalEntry> {
    // Parses the list of durations in the format XXX:nn,nnU, i.e. Mon:1,5h into Weekday, duration and unit
    let durations: Vec<(Weekday, f32, String)> = parse_worklog_durations(durations);

    let mut inserted_work_logs: Vec<JournalEntry> = vec![];

    for entry in durations.into_iter() {
        let weekday = entry.0;
        let duration = entry.1;
        let unit = entry.2;

        let started = date_of_last_weekday(weekday);
        // Starts all entries at 08:00
        let started = Local
            .with_ymd_and_hms(started.year(), started.month(), started.day(), 8, 0, 0)
            .unwrap();

        let started = started.format("%Y-%m-%dT%H:%M").to_string();
        let duration = format!("{}{}", duration, unit);
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
    issue: String,
    duration: &str,
    started: Option<String>,
    comment: Option<String>,
) -> JournalEntry {
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
        Err(e) => {
            eprintln!("Unable to figure out the duration of your worklog entry from '{}', error message is: {}", duration, e);
            exit(4);
        }
    };
    debug!("time spent in seconds: {}_", time_spent_seconds);

    // If a starting point was given, transform it from string to a full DateTime<Local>
    let starting_point = started.as_ref().map(|dt| str_to_date_time(dt).unwrap());
    // Optionally calculates the starting point after which it is verified
    let calculated_start = calculate_started_time(starting_point, time_spent_seconds)
        .unwrap_or_else(|err: DateTimeError| {
            eprintln!("{}", err);
            exit(4);
        });

    println!("Using these parameters as input:");
    println!("\tIssue: {}", issue.as_str());
    println!(
        "\tStarted: {}  ({})",
        calculated_start.to_rfc3339(),
        started.map_or("computed", |_| "computed from command line")
    );
    println!("\tDuration: {}s", time_spent_seconds);
    println!("\tComment: {}", comment.as_deref().unwrap_or("None"));

    let result = match jira_client
        .insert_worklog(
            issue.as_str(),
            calculated_start,
            time_spent_seconds,
            comment.as_deref().unwrap_or(""),
        )
        .await
    {
        Ok(result) => result,
        Err(e) => match e {
            StatusCode::NOT_FOUND => {
                eprintln!("WARNING: Issue {} not found", issue);
                exit(4);
            }
            other => {
                eprintln!(
                    "ERROR: Unable to insert worklog entry for issue {}, http error code {}",
                    issue, other
                );
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
    println!("To delete entry: jira_worklog del -i {} -w {}", issue, &result.id);

    JournalEntry {
        issue_key: issue,
        worklog_id: result.id,
        started: calculated_start.with_timezone(&Local),
        time_spent_seconds: result.timeSpentSeconds,
        comment
    }

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
        date_util::seconds_to_hour_and_min(sum_per_week)
    );
    println!("{:=<23}", "");
    println!();
}