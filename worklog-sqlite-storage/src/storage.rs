//! `SQLite` storage implementation

use rusqlite::Connection;
use std::borrow::Cow;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use worklog_core::{WorklogError, WorklogStorage};

/// Shared `SQLite` connection type
pub(crate) type SharedSqliteConnection = Arc<Mutex<Connection>>;

/// `SQLite` implementation of the `WorklogStorage` trait.
///
/// This storage provides persistent storage using `SQLite`, which can be
/// configured to use either an on-disk database file or an in-memory database.
///
/// # Features
///
/// - **Thread-safe**: Uses `Arc<Mutex<Connection>>` for safe concurrent access
/// - **ACID compliance**: Leverages `SQLite`'s transaction support
/// - **Foreign key support**: Automatically enables foreign key constraints
/// - **Schema management**: Automatically creates tables on initialization
pub struct SqliteStorage {
    connection: SharedSqliteConnection,
}

impl SqliteStorage {
    /// Helper function to create a `SQLite` connection with error handling.
    fn create_connection<F, G>(
        connect: F,
        context: G,
    ) -> Result<SharedSqliteConnection, WorklogError>
    where
        F: FnOnce() -> rusqlite::Result<Connection>,
        G: FnOnce() -> Cow<'static, str>,
    {
        let connection = connect().map_err(|e| {
            WorklogError::StorageError(format!(
                "Failed to open SQLite database ({}): {}",
                context(),
                e
            ))
        })?;

        // Enable foreign key support
        connection
            .execute_batch("PRAGMA foreign_keys = ON")
            .map_err(|e| {
                WorklogError::StorageError(format!("Failed to enable foreign keys: {e}"))
            })?;

        Ok(Arc::new(Mutex::new(connection)))
    }

    /// Parse `SQLite` URL and apply options
    ///
    /// URL formats:
    /// - `sqlite:///absolute/path/to/db.db`
    /// - `sqlite://relative/path/to/db.db`
    /// - `sqlite://:memory:`
    fn parse_url_and_options(
        url: &str,
        options: Option<&HashMap<String, String>>,
    ) -> Result<(PathBuf, HashMap<String, String>), WorklogError> {
        if !url.starts_with("sqlite://") {
            return Err(WorklogError::ConfigError(format!(
                "Invalid SQLite URL format: {url}. Expected format: sqlite:///path/to/db.db or sqlite://:memory:"
            )));
        }

        let path_str = url.strip_prefix("sqlite://").unwrap();

        // Handle :memory: special case
        if path_str == ":memory:" {
            return Ok((
                PathBuf::from(":memory:"),
                options.cloned().unwrap_or_default(),
            ));
        }

        // Convert to PathBuf
        let path = PathBuf::from(path_str);
        let opts = options.cloned().unwrap_or_default();

        Ok((path, opts))
    }

    /// Apply SQLite-specific options via PRAGMA statements
    fn apply_options(
        connection: &Connection,
        options: &HashMap<String, String>,
    ) -> Result<(), WorklogError> {
        // Common SQLite options that can be set via PRAGMA
        if let Some(journal_mode) = options.get("journal_mode") {
            connection
                .execute_batch(&format!("PRAGMA journal_mode = {journal_mode}"))
                .map_err(|e| {
                    WorklogError::StorageError(format!("Failed to set journal_mode: {e}"))
                })?;
        }

        if let Some(cache_size) = options.get("cache_size") {
            connection
                .execute_batch(&format!("PRAGMA cache_size = {cache_size}"))
                .map_err(|e| {
                    WorklogError::StorageError(format!("Failed to set cache_size: {e}"))
                })?;
        }

        if let Some(synchronous) = options.get("synchronous") {
            connection
                .execute_batch(&format!("PRAGMA synchronous = {synchronous}"))
                .map_err(|e| {
                    WorklogError::StorageError(format!("Failed to set synchronous: {e}"))
                })?;
        }

        Ok(())
    }
}

impl WorklogStorage for SqliteStorage {
    fn from_url(
        url: &str,
        options: Option<&HashMap<String, String>>,
    ) -> Result<Self, WorklogError> {
        let (path, opts) = Self::parse_url_and_options(url, options)?;

        // Create connection based on path
        let connection = if path.to_str() == Some(":memory:") {
            Self::create_connection(Connection::open_in_memory, || "in-memory".into())?
        } else {
            let path_clone = path.clone();
            Self::create_connection(
                || Connection::open(&path),
                move || Cow::from(path_clone.to_string_lossy().into_owned()),
            )?
        };

        // Apply SQLite-specific options
        {
            let conn = connection.lock().map_err(|e| {
                WorklogError::StorageError(format!("Failed to lock connection: {e}"))
            })?;
            Self::apply_options(&conn, &opts)?;
        }

        // Initialize or migrate the database schema
        {
            let mut conn = connection.lock().map_err(|e| {
                WorklogError::StorageError(format!("Failed to lock connection: {e}"))
            })?;
            crate::migrations::initialize_or_migrate(&mut conn)
                .map_err(|e| WorklogError::StorageError(format!("Migration failed: {e}")))?;
        }

        Ok(Self { connection })
    }

    fn name(&self) -> &'static str {
        "SQLite"
    }

    fn create_worklog_repository(&self) -> Arc<dyn worklog_core::WorklogRepository> {
        Arc::new(crate::repositories::WorklogRepository::new(
            self.connection.clone(),
        ))
    }

    fn create_issue_repository(&self) -> Arc<dyn worklog_core::IssueRepository> {
        Arc::new(crate::repositories::IssueRepository::new(
            self.connection.clone(),
        ))
    }

    fn create_user_repository(&self) -> Arc<dyn worklog_core::UserRepository> {
        Arc::new(crate::repositories::UserRepository::new(
            self.connection.clone(),
        ))
    }
}

// Convenience methods for direct access to repositories
impl SqliteStorage {
    /// Create a worklog repository (convenience method)
    #[must_use]
    pub fn worklog_repository(&self) -> Arc<dyn worklog_core::WorklogRepository> {
        self.create_worklog_repository()
    }

    /// Create an issue repository (convenience method)
    #[must_use]
    pub fn issue_repository(&self) -> Arc<dyn worklog_core::IssueRepository> {
        self.create_issue_repository()
    }

    /// Create a user repository (convenience method)
    #[must_use]
    pub fn user_repository(&self) -> Arc<dyn worklog_core::UserRepository> {
        self.create_user_repository()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sqlite_storage_in_memory() -> Result<(), WorklogError> {
        let storage = SqliteStorage::from_url("sqlite://:memory:", None)?;

        // Verify we can create repositories
        let _ = storage.create_user_repository();
        let _ = storage.create_worklog_repository();
        let _ = storage.create_issue_repository();

        assert_eq!(storage.name(), "SQLite");

        Ok(())
    }

    #[test]
    fn test_sqlite_storage_with_options() -> Result<(), WorklogError> {
        let mut opts = HashMap::new();
        opts.insert("journal_mode".to_string(), "WAL".to_string());
        opts.insert("cache_size".to_string(), "10000".to_string());

        let storage = SqliteStorage::from_url("sqlite://:memory:", Some(&opts))?;

        assert_eq!(storage.name(), "SQLite");

        Ok(())
    }

    #[test]
    fn test_sqlite_storage_invalid_url() {
        let result = SqliteStorage::from_url("postgres://localhost/db", None);
        assert!(result.is_err());
    }
}
