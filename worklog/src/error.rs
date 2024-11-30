use std::{io, path::PathBuf};

use jira::JiraError;
use thiserror::Error;

use crate::date;

#[allow(clippy::module_name_repetitions)]
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
