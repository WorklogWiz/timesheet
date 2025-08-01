//! # The Jira Worklog Command Line Utility
//!
//! A command-line tool for managing Jira work log entries. Supports adding, deleting,
//! and listing work logs, as well as synchronizing with Jira servers.
//!
//! ## Configuration
//! Before using the tool, configure it with your Jira credentials:
//! ```bash
//! timesheet config update --token YOUR_API_TOKEN --user your.email@company.com --url https://yourcompany.atlassian.net/rest/api/latest
//! ```
//!
//! ## Usage Examples
//!
//! ### Adding Work Logs
//! Add a single work log:
//! ```bash
//! timesheet add -i PROJ-123 -d 4h -s 2024-02-01 -c "Implemented feature X"
//! ```
//!
//! Add multiple work logs for different days:
//! ```bash
//! timesheet add -i PROJ-123 -d Mon:4h Tue:3.5h Wed:6h
//! ```
//!
//! ### Deleting Work Logs
//! ```bash
//! timesheet del -i PROJ-123 -w 12345
//! ```
//!
//! ### Viewing Status
//! View work logs for specific issues:
//! ```bash
//! timesheet status -i PROJ-123 PROJ-124 --start-after 2024-01-01
//! ```
//!
//! ### Synchronizing with Jira
//! Sync current month's work logs:
//! ```bash
//! timesheet sync
//! ```
//!
//! Sync specific projects:
//! ```bash
//! timesheet sync -p PROJ TIME --all-users
//! ```
//!
//! ### Listing Time Codes from Jira project TIME
//! List all time codes from Jira project named `TIME`:
//!
//! ```bash
//! timesheet codes
//! ```
//!
//! ## Time Format
//! - Hours: 4h, 1.5h, 1,5h
//! - Days: 1d
//! - Combined: 7h30m
//! - Time format: 7:30 (7 hours 30 minutes)
//!
use chrono::Local;
use clap::Parser;
use cli::{Command, LogLevel, Opts};
use commands::{configuration, status};
use env_logger::Env;
use log::debug;
use std::env;
use std::fs::File;
use std::process::exit;

use worklog::{
    date, error::WorklogError, operation, ApplicationRuntime, Operation, OperationResult,
};

mod cli;
mod commands;
mod table_report_weekly;

use commands::stop_timer;
use jira::models::core::IssueKey;

#[tokio::main]
#[allow(clippy::too_many_lines)] // TODO: fix this
async fn main() -> Result<(), WorklogError> {
    let opts: Opts = Opts::parse();

    configure_logging(&opts); // Handles the -v option

    #[allow(clippy::match_wildcard_for_single_variants)]
    match opts.cmd {
        Command::Add(add_cmd) => {
            let or: &worklog::OperationResult = &get_runtime()
                .execute(Operation::Add(add_cmd.into()))
                .await?;
            match or {
                worklog::OperationResult::Added(items) => {
                    for item in items {
                        println!(
                            "Added work log entry Id: {} Time spent: {} Time spent in seconds: {} Comment: {}",
                            &item.id,
                            &item.timeSpent,
                            &item.timeSpentSeconds,
                            &item.comment.as_deref().unwrap_or("")
                        );
                        println!(
                            "To delete entry: timesheet del -i {} -w {}",
                            &item.issue_key, &item.id
                        );
                    }
                }
                _ => panic!("This should never happen!"),
            }
        }

        Command::Del(del) => {
            let operation_result = &get_runtime().execute(Operation::Del(del.into())).await?;
            match operation_result {
                worklog::OperationResult::Deleted(id) => {
                    println!("Jira work log id {id} deleted from Jira");
                }
                _ => todo!(),
            }
        }

        Command::Status(status) => {
            status::execute(status).await?;
        }

        Command::Config(config) => {
            configuration::execute(config.cmd);
        } // end Config
        Command::Codes => {
            let operation_result: &worklog::OperationResult =
                &get_runtime().execute(Operation::Codes).await?;
            match operation_result {
                worklog::OperationResult::IssueSummaries(issues) => {
                    for issue in issues {
                        println!("{} {}", issue.key, issue.fields.summary);
                    }
                }
                _ => todo!(),
            }
        }
        Command::Sync(sync_cmd) => {
            let operation_result: &worklog::OperationResult = &get_runtime()
                .execute(Operation::Sync(sync_cmd.into()))
                .await?;
            match operation_result {
                OperationResult::Synchronised => {}
                _ => {
                    unimplemented!()
                }
            }
        }
        Command::Start(start_opts) => {
            // TODO: refactor this into a separate module `commands::start_timer`
            // Determine the start time
            let start = match start_opts.start {
                None => Local::now(),
                Some(supplied_dt_string) => date::str_to_date_time(&supplied_dt_string)
                    .unwrap_or_else(|err| {
                        eprintln!("Unable to parse date/time: {err}");
                        exit(1);
                    }),
            };

            match &get_runtime()
                .timer_service
                .start_timer(&start_opts.issue, start, start_opts.comment)
                .await
            {
                Ok(timer) => {
                    let issue_summary = &get_runtime()
                        .issue_service
                        .get_issues_filtered_by_keys(&[IssueKey::new(&timer.issue_key)])
                        .ok()
                        .and_then(|issues| issues.first().cloned())
                        .unwrap();
                    println!(
                        "Started timer for issue {} - '{}' with id {:?} at {}",
                        &start_opts.issue,
                        &issue_summary.summary,
                        timer.id.as_ref().unwrap(),
                        timer.started_at.format("%Y-%m-%d %H:%M")
                    );
                }
                Err(e) => {
                    println!(
                        "Unable to start timer for issue {}. Cause: {e}",
                        start_opts.issue
                    );
                }
            }
        }
        Command::Stop(stop_opts) => {
            if stop_opts.discard {
                return stop_timer::discard_active_timer(&get_runtime());
            }

            let stop_time = stop_timer::parse_stop_time(stop_opts.stopped_at.as_deref());
            let _ = stop_timer::stop_timer(&get_runtime(), stop_time, stop_opts.comment.clone());

            stop_timer::sync_timers_to_jira(&get_runtime()).await?;
        } // Stop
    }
    Ok(())
}

/// Retrieves the application configuration file
fn get_runtime() -> ApplicationRuntime {
    match ApplicationRuntime::new() {
        Ok(runtime) => runtime,
        Err(err) => {
            match err {
                WorklogError::ApplicationConfig { .. } => {
                    eprintln!(
                        "Configuration file not found. Use 'timesheet config update' to create it"
                    );
                }
                _ => {
                    eprintln!("Failed to create runtime: '{err}'");
                }
            }

            exit(1);
        }
    }
}

fn configure_logging(opts: &Opts) {
    let mut tmp_dir = env::temp_dir();
    tmp_dir.push("timesheet.log");

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

impl From<cli::Add> for operation::add::Add {
    fn from(val: cli::Add) -> Self {
        operation::add::Add {
            durations: val.durations,
            issue_key: val.issue,
            started: val.started,
            comment: val.comment,
        }
    }
}
