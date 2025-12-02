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
//! ```bash
//! timesheet status
//! ```
//!
//! ### Timer Operations
//! Start a timer for an issue:
//! ```bash
//! timesheet start -i PROJ-123 -c "Working on feature"
//! ```
//!
//! Stop the active timer:
//! ```bash
//! timesheet stop
//! ```
//!
//! Discard the active timer without saving:
//! ```bash
//! timesheet stop --discard
//! ```
//!
//! ### Synchronizing with Jira
//! ```bash
//! timesheet sync
//! ```
//!
//! ### Listing Issue Codes
//! ```bash
//! timesheet codes
//! ```

use clap::Parser;
use cli::{Command, LogLevel, Opts};
use commands::{add, codes, configuration, del, start_timer, status, stop_timer, sync};
use env_logger::Env;
use log::debug;
use std::env;
use std::fs::File;

use worklog_core::WorklogError;

mod cli;
mod commands;
mod date_utils;
mod services;
mod table_report_weekly;

#[tokio::main]
#[allow(clippy::too_many_lines)] // TODO: fix this
async fn main() -> Result<(), WorklogError> {
    let opts: Opts = Opts::parse();

    configure_logging(&opts); // Handles the -v option

    #[allow(clippy::match_wildcard_for_single_variants)]
    match opts.cmd {
        Command::Add(add_cmd) => {
            let services = services::get_services()?;
            let options = add::AddOptions {
                durations: add_cmd.durations,
                issue_key: add_cmd.issue,
                started: add_cmd.started,
                comment: add_cmd.comment,
            };
            add::execute(&services, options).await?;
        }

        Command::Del(del) => {
            let services = services::get_services()?;
            let options = del::DeleteOptions {
                issue_key: del.issue_id,
                worklog_id: del.worklog_id,
            };
            del::execute(&services, options).await?;
        }

        Command::Status(status) => {
            status::execute(status).await?;
        }

        Command::Config(config) => {
            configuration::execute(config.cmd);
        } // end Config

        Command::Codes(codes_cmd) => {
            let services = services::get_services()?;
            let options = codes::CodesOptions {
                projects: codes_cmd.projects,
                all_users: codes_cmd.all_users,
                all: codes_cmd.all,
            };
            let issues = codes::execute(&services, options).await?;
            for issue in issues {
                println!("{}: {}", issue.key, issue.summary);
            }
        }

        Command::Sync(sync_cmd) => {
            let services = services::get_services()?;
            let options = sync::SyncOptions {
                started: sync_cmd.started,
                all_users: sync_cmd.all_users,
                projects: sync_cmd.projects,
                issues: sync_cmd.issues,
            };
            sync::execute(&services, options).await?;
        }

        Command::Start(start_opts) => {
            let services = services::get_services()?;
            let options = start_timer::StartTimerOptions {
                issue: start_opts.issue,
                comment: start_opts.comment,
                start_time: start_opts.start,
            };
            start_timer::start_timer(&services, options).await?;
        }

        Command::Stop(stop_opts) => {
            let services = services::get_services()?;

            if stop_opts.discard {
                return stop_timer::discard_active_timer(&services).await;
            }

            let stop_time = stop_opts.stopped_at.unwrap_or_else(chrono::Local::now);
            stop_timer::stop_timer(&services, stop_time, stop_opts.comment.clone()).await?;
        } // Stop
    }
    Ok(())
}

fn configure_logging(opts: &Opts) {
    let mut tmp_dir = env::temp_dir();
    tmp_dir.push("timesheet.log");

    if opts.verbosity.is_some() {
        println!("Logging to {}", &tmp_dir.to_string_lossy());
    }

    let mut env_builder = Env::default();

    if let Some(log_level) = opts.verbosity {
        let level = match log_level {
            LogLevel::Debug => "debug",
            LogLevel::Info => "info",
            LogLevel::Warn => "warn",
            LogLevel::Error => "error",
        };
        env_builder = env_builder.default_filter_or(level);
    } else {
        env_builder = env_builder.default_filter_or("error");
    }

    env_logger::Builder::from_env(env_builder)
        .target(env_logger::Target::Pipe(Box::new(
            File::create(tmp_dir).expect("Failed to create log file"),
        )))
        .init();
    debug!("Log level set");
}
