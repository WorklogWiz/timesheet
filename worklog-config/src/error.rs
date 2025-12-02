use std::{io, path::PathBuf};

use thiserror::Error;

/// Error type for configuration loading
///
/// This crate is primarily responsible for loading and managing application configuration.
/// All domain logic errors use `worklog_core::WorklogError` directly.
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

    #[error("Directory creation failed")]
    CreateDir(#[from] io::Error),
}
