use std::fmt::{self, Formatter};

use clap::{Args, Parser, Subcommand, ValueEnum};

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
#[command(author, version, about)] // Read from Cargo.toml
pub(crate) struct Opts {
    #[command(subcommand)]
    pub subcmd: SubCommand,

    #[arg(global = true, short, long)]
    pub verbosity: Option<LogLevel>,
}

#[derive(Subcommand)]
pub(crate) enum SubCommand {
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
pub(crate) struct Add {
    /// Duration of work in hours (h) or days (d)
    /// If more than a single entry separate with spaces and three letter abbreviation of
    /// weekday name:
    ///     --durations Mon:1,5h Tue:1d Wed:3,5h Fri:1d
    #[arg(short, long, num_args(1..))]
    pub durations: Vec<String>,
    /// Jira issue to register work on
    #[arg(short, long, required = true)]
    pub issue: String,
    /// work started
    #[arg(name = "started", short, long, requires = "durations")]
    pub started: Option<String>,
    #[arg(name = "comment", short, long)]
    pub comment: Option<String>,
}

#[derive(Args)]
pub(crate) struct Del {
    #[arg(short, long, required = true)]
    pub issue_id: String,
    #[arg(short = 'w', long, required = true)]
    pub worklog_id: String,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
pub(crate) enum LogLevel {
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

#[derive(Parser)]
pub(crate) struct Status {
    /// Issues to be reported on. If no issues are specified.
    /// The unique Jira keys found in the local journal of entries is used.
    /// You can specify a list of issue keys: -i time-147 time-148
    #[arg(short, long, num_args(1..), required = false)]
    pub issues: Option<Vec<String>>,
    #[arg(short, long)]
    /// Retrieves all entries after the given date
    pub after: Option<String>,
}

/// Create, modify or list the configuration file.
/// The configuration file will be automatically created if you use `--token`, `--user` or `--jira_url`
#[derive(Parser)]
pub(crate) struct Configuration {
    /// The Jira security API token obtained from your Manage Account -> Security
    #[arg(short, long)]
    pub token: Option<String>,
    /// Your email address, i.e. me@whereever.com
    #[arg(short, long)]
    pub user: Option<String>,
    /// Lists the current configuration (if it exists) and exit
    #[arg(short, long)]
    pub list: bool,
    /// The URL of Jira, don't change this unless you know what you are doing
    #[arg(
        short,
        long,
        default_value = "https://autostore.atlassian.net/rest/api/latest"
    )]
    pub jira_url: Option<String>,
    /// The name of the project where the issues to track time on are (make it a list?)
    #[arg(short, long, default_value = "TIME")]
    pub tracking_project: Option<String>,
    /// Remove the current configuration
    #[arg(long, default_value_t = false)]
    pub remove: bool,
}

#[derive(Parser)]
pub(crate) struct Synchronisation {
    #[arg(name = "started", short, long)]
    /// Default is to sync for the current month, but you may specify an ISO8601 date from which
    /// data should be synchronised
    pub started: Option<String>,
    #[arg(
        name = "issues",
        short,
        long,
        long_help = "Limit synchronisation to these issues"
    )]
    pub issues: Vec<String>,
}
