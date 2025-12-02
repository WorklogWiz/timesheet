use std::fmt::{self, Formatter};

use chrono::{DateTime, Local};
use clap::{ArgGroup, Args, Parser, Subcommand, ValueEnum};

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
/// Worklog utility - add, delete, and list worklog entries
///
/// Dates should be specified in the ISO8601 format without a time zone. Local timezone is
/// always assumed. I.e. `2023-06-01`.
///
/// Duration is specified in units of hours, days, or weeks, using the abbreviations 'h','d', and 'w'
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
    pub cmd: Command,

    #[arg(global = true, short, long)]
    pub verbosity: Option<LogLevel>,
}

#[derive(Subcommand)]
pub(crate) enum Command {
    /// Add worklog entries
    Add(Add),
    /// Delete work log entry
    Del(Del),
    /// Get status of work log entries
    Status(Status),
    /// Subcommands for configuration
    Config(Config),
    /// Lists all time codes
    Codes(Codes),
    /// Start a timer
    Start(Start),
    /// Stops current timer
    Stop(Stop),
    /// Synchronize the local data store with remote Jira work logs
    Sync(Synchronisation),
}

#[derive(Args)]
pub(crate) struct Add {
    /// Duration of work in hours (h) or days (d)
    /// If more than a single entry separate with spaces and three letter abbreviations of
    /// weekday name:
    ///     --durations Mon:1,5h Tue:1d Wed:3,5h Fri:1d
    #[arg(short, long, num_args(1..), value_parser = crate::date_utils::DurationParser)]
    pub durations: Vec<crate::date_utils::DurationEntry>,
    /// Jira issues to register work on
    #[arg(short, long, required = true)]
    pub issue: String,
    /// work started
    #[arg(name = "started", short, long, requires = "durations", value_parser = crate::date_utils::DateTimeParser)]
    pub started: Option<DateTime<Local>>,
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

#[derive(Args)]
pub(crate) struct Status {
    /// Issues to be reported on. If no issues are supplied,
    /// the unique Jira keys found in the local journal of entries is used.
    /// You can specify a list of issue keys: -i time-147 time-148
    #[arg(short, long, num_args(1..), required = false)]
    pub issues: Option<Vec<String>>,
    /// Retrieves all entries after the given date
    #[arg(short, long, value_parser = crate::date_utils::DateTimeParser)]
    pub start_after: Option<DateTime<Local>>,
    /// Reports on all registered Jira users, not just you
    #[arg(short, long)]
    pub all_users: bool,
}

#[derive(Args)]
pub(crate) struct Config {
    #[command(subcommand)]
    pub cmd: ConfigCommand,
}

/// Create, modify, or list the configuration file.
/// The configuration file will be automatically created if you use `--token`, `--user` or `--url`
#[derive(Subcommand, Clone)]
pub(crate) enum ConfigCommand {
    /// Update the configuration file
    Update(UpdateConfiguration),
    /// Writes the current configuration to standard output
    List,
    /// Remove the current configuration
    Remove,
}

#[derive(Args, Clone)]
pub(crate) struct UpdateConfiguration {
    /// The Jira security API token obtained from your Manage Account -> Security
    #[arg(short, long)]
    pub token: String,
    /// Your email address, i.e. me@whereever.com
    #[arg(short, long)]
    pub user: String,
    /// The base url to your Jira, typically <https://yourcompany.atlassian.net/rest/api/latest>
    #[arg(long)]
    pub url: String,
    // TODO: replace tracking_project with "projects" in the plural
    #[arg(long, default_value = "TIME")]
    pub tracking_project: String,
}

#[derive(Args)]
#[clap(group(
    ArgGroup::new("user_filter")
        .args(["all_users", "all"])
))]
pub(crate) struct Codes {
    /// Project(s) to filter by. If not specified, defaults to "TIME"
    #[arg(short, long)]
    pub projects: Vec<String>,
    /// Show issues where any user has logged time (not just you)
    #[arg(long)]
    pub all_users: bool,
    /// Show ALL issues in the project(s), regardless of whether anyone has logged time
    #[arg(short, long)]
    pub all: bool,
}

#[derive(Args)]
pub(crate) struct Synchronisation {
    #[arg(name = "started", short, long, value_parser = crate::date_utils::DateTimeParser)]
    /// The default is to sync for the current month, but you may specify an ISO8601 date from which
    /// data should be synchronised
    pub started: Option<DateTime<Local>>,
    #[arg(
        name = "issues",
        short,
        long,
        long_help = "Limit synchronisation to these issues",
        group = "sync_targets"
    )]
    pub issues: Vec<String>,
    /// Synchronise all work logs for all issues in the list of projects
    #[arg(
        name = "projects",
        short,
        long,
        long_help = "Limit synchronisation to these projects",
        group = "sync_targets"
    )]
    pub projects: Vec<String>,
    /// Retrieves all registered Jira users, not just you
    #[arg(short, long)]
    pub all_users: bool,
}

#[derive(Args)]
pub(crate) struct Start {
    #[arg(short, long, long_help = "Issue to start timer on")]
    pub issue: String,
    #[arg(short, long, long_help = "Comment to add to work log")]
    pub comment: Option<String>,
    #[arg(
        short,
        long,
        long_help = "Starting point if different from current time",
        value_parser = crate::date_utils::DateTimeParser
    )]
    #[allow(clippy::struct_field_names)]
    pub start: Option<DateTime<Local>>,
}

#[derive(Args)]
#[clap(group(
    ArgGroup::new("normal_stop")
        .args(["stopped_at", "comment"])
        .conflicts_with("discard")
        .multiple(true)
))]
pub(crate) struct Stop {
    /// The stopping time to use rather than the current time, which is the default
    #[arg(
        short,
        long,
        long_help = "Stop timer at this time rather than current time",
        value_parser = crate::date_utils::DateTimeParser
    )]
    pub stopped_at: Option<DateTime<Local>>,
    /// Comment to add to the work log entry, will overwrite the comment given on start
    #[arg(
        short,
        long,
        long_help = "Comment to add to work log (overwrites comment given on start"
    )]
    pub comment: Option<String>,
    #[arg(short, long, long_help = "Discard the active work log entry")]
    pub discard: bool,
}
