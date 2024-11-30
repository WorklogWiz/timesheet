//! The Jira worklog command line utility
//!
use std::env;
use std::fs::File;
use std::process::exit;

use clap::Parser;
use cli::{LogLevel, Opts, SubCommand};
use commands::{add, configuration, del, status, sync};
use env_logger::Env;
use log::{debug, info};

use jira_lib::Jira;
use worklog_lib::{config, ApplicationRuntime};

mod cli;
mod commands;
mod table_report_weekly;

const VERSION: &str = env!("CARGO_PKG_VERSION");

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
            add::execute(&mut add).await;
        }

        SubCommand::Del(delete) => {
            del::execute(&delete).await;
        }

        SubCommand::Status(status) => {
            status::execute(status).await;
        }

        SubCommand::Config(config) => {
            configuration::execute(config);
        } // end Config
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
            let _result = sync::execute(synchronisation).await;
        }
    }
}

/// Creates the `JiraClient` instance based on the supplied parameters.
fn get_jira_client(app_config: &config::AppConfiguration) -> Jira {
    match Jira::new(
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
