use std::fs::File;
use std::path::PathBuf;
use std::{env, io};

use env_logger::Env;
use log;
use log::{debug, Level};
use rusqlite::Error;
use thiserror::Error;

pub mod config;
pub mod date;
pub mod journal;

pub fn configure_logging(log_level: log::Level) {
    let mut tmp_dir = env::temp_dir();
    tmp_dir.push("jira_worklog.log");

    let _target = Box::new(File::create(tmp_dir).expect("Can't create file"));

    // If nothing else was specified in RUST_LOG, use 'warn'
    env_logger::Builder::from_env(Env::default().default_filter_or(match log_level {
        Level::Debug => "debug",
        Level::Info => "info",
        Level::Warn => "warn",
        Level::Error => "error",
        Level::Trace => "trace",
    }))
    // .target(env_logger::Target::Pipe(target))
    .init();
    debug!("Logging started");
}

#[derive(Error, Debug)]
pub enum WorklogError {
    #[error("Unable to load the application configuration file {path:?}")]
    ApplicationConfig { path: PathBuf, source: io::Error },
    #[error("Unable to parse contents of {path}")]
    TomlParse {
        path: PathBuf,
        source: toml::de::Error,
    },
    #[error("Unable to create configuration file {path}")]
    ConfigFileCreation { path: PathBuf },
    #[error("Error creating Jira client: {msg}")]
    JiraClient { msg: String },
    #[error("Jira request failed: {msg} : {reason}" )]
    JiraResponse{ msg: String, reason: String},
    #[error("Unable to open journal file {0}")]
    OpenJournal(String),
    #[error("Unable to open DBMS in file {path}: {reason}")]
    OpenDbms { path: String, reason: String},
    #[error("Unable to create file: {0}")]
    CreateFile (String),
    #[error("SQL dbms error: {0}")]
    Sql(String),
    #[error("Unable to delete file {0}, are you sure it is not locked?")]
    FileNotDeleted(String),
    #[error("Directory creation failed")]
    CreateDir(#[from] io::Error),
    #[error("Unable to retrieve the unique jira keys from the deprecated local journal: {0}")]
    UniqueKeys(String),
    #[error("Invalid Jira token in application configuration struct")]
    InvalidJiraToken,
    #[error("File not found: {0}")]
    FileNotFound(String),
}

impl From<rusqlite::Error> for WorklogError {
    fn from(err: rusqlite::Error) -> Self {
        match err {
                  _ =>   WorklogError::Sql(format!("Sqlite error {}", err.to_string())) ,
        }
    }
}

