//! Storage abstraction trait

use crate::error::WorklogResult;
use crate::traits::{IssueRepository, UserRepository, WorklogRepository};
use std::collections::HashMap;
use std::sync::Arc;

/// Trait defining the interface for worklog storage implementations.
///
/// Any storage implementation (`SQLite`, `PostgreSQL`, `MongoDB`, etc.) must
/// implement this trait to be usable with the worklog system.
///
/// # Connection URL Format
///
/// The `url` parameter follows standard database URL patterns:
///
/// ## SQL Databases
/// - `SQLite`: `sqlite:///path/to/db.db` or `sqlite://:memory:`
/// - `PostgreSQL`: `postgres://user:pass@host:port/database`
/// - `MySQL`: `mysql://user:pass@host:port/database`
///
/// ## `NoSQL` Databases
/// - `MongoDB`: `mongodb://host:port/database` or `mongodb+srv://cluster/database`
/// - Redis: `redis://host:port/db`
/// - `CouchDB`: `couchdb://host:port/database`
///
/// ## Cloud Services
/// - `DynamoDB`: `dynamodb://region/table` (requires AWS credentials in options)
/// - Firestore: `firestore://project-id/collection`
///
/// # Options
///
/// Provider-specific configuration passed as key-value pairs:
/// - Connection pool settings
/// - Timeouts
/// - Cloud provider credentials
/// - Provider-specific features (WAL mode, replication, etc.)
///
/// # Example
///
/// ```ignore
/// use worklog_core::traits::WorklogStorage;
/// use std::collections::HashMap;
///
/// // Simple case - just URL
/// let storage = SqliteStorage::from_url("sqlite:///path/to/db.db", None)?;
///
/// // With options
/// let mut opts = HashMap::new();
/// opts.insert("journal_mode".to_string(), "WAL".to_string());
/// let storage = SqliteStorage::from_url("sqlite:///path/to/db.db", Some(&opts))?;
/// ```
pub trait WorklogStorage: Send + Sync {
    /// Create a new storage instance from a connection URL
    ///
    /// # Arguments
    ///
    /// * `url` - Connection URL (format depends on storage provider)
    /// * `options` - Optional provider-specific configuration
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - URL format is invalid
    /// - Connection fails
    /// - Schema initialization fails
    /// - Required options are missing
    fn from_url(url: &str, options: Option<&HashMap<String, String>>) -> WorklogResult<Self>
    where
        Self: Sized;

    /// Create a worklog repository
    fn create_worklog_repository(&self) -> Arc<dyn WorklogRepository>;

    /// Create an issue repository
    fn create_issue_repository(&self) -> Arc<dyn IssueRepository>;

    /// Create a user repository
    fn create_user_repository(&self) -> Arc<dyn UserRepository>;

    /// Returns a human-readable name for this storage implementation
    ///
    /// Useful for logging and debugging.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let storage = SqliteStorage::from_url("sqlite://:memory:", None)?;
    /// assert_eq!(storage.name(), "SQLite");
    /// ```
    fn name(&self) -> &'static str;
}
