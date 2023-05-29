//! # The Jira worklog command line utility
//!
use std::{env, fmt};
use std::fmt::Formatter;
use std::fs::File;
use clap::{Args, Parser, Subcommand, ValueEnum};
use env_logger::Env;
use crate::date_util::{calculate_started_time, str_to_date_time, TimeSpent};
use log::{debug, info};
use jira_lib::config;
mod date_util;

#[derive(Parser)]
#[command(version = "1.0", author = "Steinar Overbeck Cook <steinar.cook@autostoresystem.com>", about = "Command line tool for Jira worklog mgmt")]
struct Opts {
    #[command(subcommand)]
    subcmd: SubCommand,

    #[arg(global=true, short, long, default_value = "info")]
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
    Del(Del),
    Get(Get),
}

#[derive(Args)]
struct Add {
    /// Duration of work in hours (h) or days (d)
    #[arg(short, long, default_value = "1d")]
    duration: String,
    /// Jira issue to register work on
    #[arg(short, long)]
    issue: String,
    /// work started
    #[arg(name = "started", short, long, requires = "duration")]
    started: Option<String>,
    /// work ended
    #[arg(name = "ended", short, long)]
    ended: Option<String>,
    #[arg(name = "comment", short, long)]
    comment: Option<String>,
    #[arg(long)]
    dry: bool,
}

#[derive(Args)]
struct Del {}

#[derive(Parser)]
struct Get {
    num1: i32,
    num2: i32,
}

#[tokio::main]
async fn main() {
    let opts: Opts = Opts::parse();

    configure_logging(&opts);

    let configuration = match config::load_configuration() {
        Ok(c) => c,
        Err(e) => { panic!("Unable to load configuration file from: {}, cause: {}", config::config_file_name().to_string_lossy(), e)}
    };

    debug!("jira_url: '{}'", configuration.jira.jira_url);
    debug!("user: '{}'", configuration.jira.user);
    debug!("token: '{}'", configuration.jira.token);

    let jira_client = match jira_lib::JiraClient::new(&configuration.jira.jira_url, &configuration.jira.user, &configuration.jira.token){
        Ok(client) => client,
        Err(e) => { panic!("Unable to create a new http-client for Jira: {}", e)}
    };

    let time_tracking_options = jira_client.get_time_tracking_options().await;
    info!("Global Jira options: {:?}", &time_tracking_options);

    match opts.subcmd {
        SubCommand::Add(add) => {
            info!("started: {}, ended: {:?}, duration:{} ", add.started.as_deref().unwrap_or("None"), add.ended, add.duration);

            let time_spent_seconds = match TimeSpent::from_str(add.duration.as_str(), time_tracking_options.workingHoursPerDay, time_tracking_options.workingDaysPerWeek) {
                Ok(time_spent) => time_spent.time_spent_seconds,
                Err(e) => panic!("Unable to figure out the duration of your worklog entry {}", e),
            };

            // If a starting point was given, transform it from string to a full DateTime<Local>
            let starting_point = add.started.as_ref().map(|dt| str_to_date_time(dt).unwrap());

            let calculated_start = calculate_started_time(starting_point, time_spent_seconds).unwrap();

            // TODO: find the starting point by subtracting duration from now()

            println!("Using these parameters as input:");
            println!("\tIssue: {}", add.issue.as_str());
            println!("\tStarted: {}  ({})", calculated_start.to_rfc3339(), add.started.map_or("computed", |_| "computed from command line"));
            println!("\tDuration: {}s", time_spent_seconds);
            println!("\tComment: {}", add.comment.as_deref().unwrap_or("None"));


            jira_client.insert_worklog(add.issue.as_str(),
                                     calculated_start,
                                     time_spent_seconds,
                                     add.comment.unwrap_or("".to_string()).as_str()).await;
        }
        SubCommand::Get(multiply) => {
            println!("{} * {} = {}", multiply.num1, multiply.num2, multiply.num1 * multiply.num2);
        }
        SubCommand::Del(_) => {}
    }
}

fn configure_logging(opts: &Opts) {
    let mut tmp_dir = env::temp_dir();
    tmp_dir.push("jira_worklog.log");

    let target = Box::new(File::create(tmp_dir).expect("Can't create file"));

    // If nothing else was specified in RUST_LOG, use 'warn'
    env_logger::Builder::from_env(Env::default()
        .default_filter_or(opts.verbosity.map_or("warn", |lvl| match lvl {
            LogLevel::Debug => "debug",
            LogLevel::Info => "info",
            LogLevel::Warn => "warn",
            LogLevel::Error => "error"
        })))
        .target(env_logger::Target::Pipe(target))
        .init();
}


