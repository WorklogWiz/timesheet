//! Error types for worklog domain

use thiserror::Error;

/// Result type for worklog operations
pub type WorklogResult<T> = Result<T, WorklogError>;

/// Domain errors for worklog operations
#[derive(Error, Debug)]
pub enum WorklogError {
    /// An active worklog entry (timer) already exists
    #[error("An active worklog entry already exists. Stop it before starting a new one.")]
    ActiveEntryExists,

    /// No active worklog entry (timer) found
    #[error("No active worklog entry found")]
    NoActiveEntry,

    /// Worklog entry not found
    #[error("Worklog entry not found: {0}")]
    EntryNotFound(String),

    /// Issue not found
    #[error("Issue not found: {0}")]
    IssueNotFound(String),

    /// User not found
    #[error("User not found")]
    UserNotFound,

    /// Invalid issue key format
    #[error("Invalid issue key format: {0}")]
    InvalidIssueKey(String),

    /// Validation error
    #[error("Validation error: {0}")]
    ValidationError(String),

    /// Cannot sync entry (missing required fields)
    #[error("Cannot sync worklog entry: {0}")]
    CannotSync(String),

    /// Storage/persistence error
    #[error("Storage error: {0}")]
    StorageError(String),

    /// Issue tracker communication error
    #[error("Issue tracker error: {0}")]
    IssueTrackerError(String),

    /// Authentication error
    #[error("Authentication error: {0}")]
    AuthenticationError(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// Generic error
    #[error("Error: {0}")]
    Other(String),
}

impl WorklogError {
    /// Check if this is a storage-related error
    #[must_use]
    pub fn is_storage_error(&self) -> bool {
        matches!(self, WorklogError::StorageError(_))
    }

    /// Check if this is an issue tracker-related error
    #[must_use]
    pub fn is_tracker_error(&self) -> bool {
        matches!(self, WorklogError::IssueTrackerError(_))
    }

    /// Check if this is a validation error
    #[must_use]
    pub fn is_validation_error(&self) -> bool {
        matches!(
            self,
            WorklogError::ValidationError(_) | WorklogError::InvalidIssueKey(_)
        )
    }
}
