use std::{io, path::PathBuf};

use crate::date;
use jira::builder::JiraBuilderError;
use jira::models::core::IssueKey;
use jira::JiraError;
use thiserror::Error;
use url::ParseError;

#[allow(clippy::module_name_repetitions)]
#[derive(Error, Debug)]
pub enum WorklogError {
    #[error("Unable to load the application configuration file {path}, cause: {source:?}")]
    ApplicationConfig { path: String, source: io::Error },
    #[error("Unable to parse contents of {path}")]
    TomlParse {
        path: PathBuf,
        source: Box<toml::de::Error>,
    },
    #[error("Unable to create configuration file {path}")]
    ConfigFileCreation { path: PathBuf },
    #[error("Unable to find configuration file {path}")]
    ConfigFileNotFound { path: PathBuf },
    #[error("Jira error {0}")]
    JiraError(String),
    #[error("Jira request failed: {msg} : {reason}")]
    JiraResponse { msg: String, reason: String },
    #[error("Unable to open journal file {0}")]
    OpenJournal(String),
    #[error("Unable to open DBMS in file {path}: {reason}")]
    OpenDbms { path: String, reason: String },
    #[error("Unable to create file: {0}")]
    CreateFile(String),
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
    #[error("Could not make sense of input: {0}")]
    BadInput(String),
    #[error("Unable to parse the url: {0}")]
    InvalidUrl(ParseError),
    #[error("Mutex locking error")]
    LockPoisoned,
    #[error("Unable to create database SQL schema: {0}")]
    DatabaseError(String),
    #[error("Active timer exists")]
    ActiveTimerExists,
    #[error("No active timer")]
    NoActiveTimer,
    #[error("Database lock error")]
    DatabaseLockError,
    #[error("Timer not found")]
    TimerNotFound(i64),
    #[error("Invalid timer data: {0}")]
    InvalidTimerData(String),
    #[error("Issue not found: {0}")]
    IssueNotFound(String),
    #[error("Invalid input: {0}")]
    InvalidInput(String),
    #[error("Jira build error: {0}")]
    JiraBuildError(JiraBuilderError),
    #[error("Timer duration too small: {0}s. Must be at least 1 minute.")]
    TimerDurationTooSmall(i32),
    #[error("Issue not found in local DBMS: {0}")]
    IssueNotFoundInLocalDBMS(String),
    #[error("Missing worklog parent, issue: {0} does not exist.")]
    MissingWorklogParentIssue(IssueKey),
}

impl From<rusqlite::Error> for WorklogError {
    fn from(err: rusqlite::Error) -> Self {
        WorklogError::Sql(format!("Sqlite error {err}"))
    }
}

impl From<JiraError> for WorklogError {
    fn from(err: JiraError) -> Self {
        WorklogError::JiraError(format!("{err}"))
    }
}

impl From<date::Error> for WorklogError {
    fn from(err: date::Error) -> Self {
        WorklogError::BadInput(format!("{err}"))
    }
}

impl From<ParseError> for WorklogError {
    fn from(value: ParseError) -> Self {
        WorklogError::InvalidUrl(value)
    }
}
