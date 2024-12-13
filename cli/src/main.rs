//! The Jira worklog command line utility
//!
use std::env;
use std::fs::File;
use std::process::exit;

use clap::Parser;
use cli::{LogLevel, Opts, SubCommand};
use commands::{configuration, status, sync};
use env_logger::Env;
use log::{debug, info};

use worklog::{error::WorklogError, operation, ApplicationRuntime, Operation};

mod cli;
mod commands;
mod table_report_weekly;

#[tokio::main]
async fn main() -> Result<(), WorklogError> {
    let opts: Opts = Opts::parse();

    configure_logging(&opts); // Handles the -v option

    if let Ok(entry_count) = worklog::migrate_csv_journal_to_local_worklog_dbms(None).await {
        debug!(
            "Migrated {} entries from CVS Journal to local work log DBMS",
            entry_count
        );
    } else {
        info!("No local CSV Journal entries migrated");
    }

    #[allow(clippy::match_wildcard_for_single_variants)]
    match opts.subcmd {
        SubCommand::Add(add) => {
            let or: &worklog::OperationResult =
                &get_runtime().execute(Operation::Add(add.into())).await?;
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

        SubCommand::Del(del) => {
            let or = &get_runtime().execute(Operation::Del(del.into())).await?;
            match or {
                worklog::OperationResult::Deleted(id) => {
                    println!("Jira work log id {id} deleted from Jira")
                }
                _ => todo!(),
            }
        }

        SubCommand::Status(status) => {
            status::execute(status).await?;
        }

        SubCommand::Config(config) => {
            configuration::execute(config);
        } // end Config
        SubCommand::Codes => {
            let operation_result: &worklog::OperationResult =
                &get_runtime().execute(Operation::Codes).await?;
            match operation_result {
                worklog::OperationResult::Issues(issues) => {
                    for issue in issues {
                        println!("{} {}", issue.key, issue.fields.summary);
                    }
                }
                _ => todo!(),
            }
        }
        SubCommand::Sync(synchronisation) => {
            sync::execute(synchronisation).await?;
        }
    }

    Ok(())
}

/// Retrieves the application configuration file
fn get_runtime() -> ApplicationRuntime {
    match ApplicationRuntime::new() {
        Ok(runtime) => runtime,
        Err(err) => {
            println!("Unable to load application runtime configuration {err}");
            println!("Create it with: timesheet config --user <EMAIL> --token <JIRA_TOKEN>");
            println!("See 'config' subcommand for more details");
            exit(4);
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
            issue: val.issue,
            started: val.started,
            comment: val.comment,
        }
    }
}

impl From<cli::Del> for operation::del::Del {
    fn from(val: cli::Del) -> Self {
        operation::del::Del {
            issue_id: val.issue_id,
            worklog_id: val.worklog_id,
        }
    }
}
