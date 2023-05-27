//! # The Jira worklog command line utility
//!
use std::ops::Deref;
use chrono::{DateTime, Local, ParseError, ParseResult};
use clap::{Args, Parser, Subcommand};
use jira_lib::TimeTrackingOptions;
use jira_lib::http_client;
use reqwest::Client;
use crate::date_util::{as_date_time, calculate_started_time, TimeSpent};

mod date_util;

#[derive(Parser)]
#[command(version = "1.0", author = "Steinar Overbeck Cook <steinar.cook@autostoresystem.com>", about = "Command line tool for Jira worklog mgmt")]
struct Opts {
    #[command(subcommand)]
    subcmd: SubCommand,

    #[clap(short, long)]
    after: Option<String>,
}

#[derive(Subcommand)]
enum SubCommand {
    /// Add worklog entries
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


    let http_client = jira_lib::http_client();
    let options = time_tracking_options(&http_client).await;
    println!("Global Jira options: {:?}", &options);

    match opts.subcmd {
        SubCommand::Add(add) => {
            println!("started: {}, ended: {:?}, duration:{} ", add.started.as_deref().unwrap_or("None"), add.ended, add.duration);

            let time_spent_seconds = match TimeSpent::from_str(add.duration.as_str(), options.workingHoursPerDay, options.workingDaysPerWeek) {
                Ok(time_spent) => time_spent.time_spent_seconds,
                Err(e) => panic!("Unable to figure out the duration of your worklog entry {}", e),
            };

            let starting_point = match add.started {
                None => None,
                Some(dt) => Some(as_date_time(&dt).unwrap())
            };

            let calculated_start = calculate_started_time(starting_point, time_spent_seconds).unwrap();

            // TODO: find the starting point by subtracting duration from now()

            println!("Issue: {}", add.issue.as_str());
            println!("Started: {} ", calculated_start.to_rfc3339());
            println!("Duration: {}s", time_spent_seconds);
            /*
                        let result = jira_lib::insert_worklog(&http_client,
                                                              add.issue.as_str(),
                                                              calculated_start,
                                                              time_spent_seconds,
                                                              add.comment.unwrap_or("".to_string()).as_str()).await;
            */
        }
        SubCommand::Get(multiply) => {
            println!("{} * {} = {}", multiply.num1, multiply.num2, multiply.num1 * multiply.num2);
        }
        SubCommand::Del(_) => {}
    }
}

async fn time_tracking_options(http_client: &Client) -> TimeTrackingOptions {
    jira_lib::get_time_tracking_options(http_client).await
}
