//! This module provides a `DatabaseManager` struct and related types
//! for managing and interacting with database connections and repositories.
//!
//! The `DatabaseConfig` enum defines supported database configurations,
//! including ``SQLite`` (on-disk and in-memory) and a placeholder for `MySQL`,
//! which can be extended to support other DBMS types in the future.
//!
//! The `DatabaseManager` struct serves as a high-level interface for
//! database initialization, schema setup, and repository creation,
//! abstracting the underlying database implementation details to enable
//! seamless use of repositories throughout the application.
//!
//! # Example
//!
//! ```rust,ignore
//! use std::path::PathBuf;
//! use crate::db::{DatabaseConfig, DatabaseManager};
//!
//! fn main() {
//!     let db_config = DatabaseConfig::SqliteOnDisk {
//!         path: PathBuf::from("my_database.db"),
//!     };
//!
//!     let db_manager = DatabaseManager::new(&db_config).expect("Failed to initialize database");
//!
//!     let issue_repo = db_manager.create_issue_repository();
//!     // Perform operations with `issue_repo` here...
//! }
//! ```

use crate::error::WorklogError;
use crate::repository::sqlite::sqlite_component_repo::SqliteComponentRepository;
use crate::repository::sqlite::sqlite_issue_repo::SqliteIssueRepository;
use crate::repository::sqlite::sqlite_user_repo::SqliteUserRepository;
use crate::repository::sqlite::sqlite_worklog_repo::SqliteWorklogRepository;
use crate::repository::user_repository::UserRepository;
use crate::repository::{sqlite, SharedSqliteConnection};
use rusqlite::{Connection, Result};
use std::borrow::Cow;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// A configuration enum that defines the parameters required for initializing
/// database connections.
///
/// This enum supports `SQLite` configurations (both on-disk and in-memory) and
/// contains a placeholder variant for ``MySQL``, which can be extended to support
/// additional database types in the future.
///
/// # Variants
///
/// * `SqliteOnDisk`
///   Represents an `SQLite` database stored on a physical file disk. Requires
///   a file path to specify the database location.
///
///   Example:
///   ```rust,ignore
///   use std::path::PathBuf;
///   use crate::db::DatabaseConfig;
///
///   let config = DatabaseConfig::SqliteOnDisk {
///       path: PathBuf::from("example.db"),
///   };
///   ```
///
/// * `SqliteInMemory`
///   Represents an `SQLite` database that operates entirely in memory. This
///   option is typically used for development, testing, or scenarios that do
///   not require data persistence.
///
///   Example:
///   ```rust,ignore
///
///   let config = DatabaseConfig::SqliteInMemory;
///   ```
///
/// * `MySql`
///   A placeholder for `MySQL` database configuration. Includes parameters
///   such as `host`, `port`, `username`, `password`, and the database name.
///   This variant is currently marked as unimplemented and serves as a
///   starting point for extending support to `MySQL` or similar DBMS types.
pub enum DatabaseConfig {
    /// `SQLite` database with a specific file path
    SqliteOnDisk { path: PathBuf },

    /// `SQLite` database that runs entirely in memory
    SqliteInMemory,

    /// A placeholder for a `MySQL` database (extendable to support other DBMS types)
    #[allow(dead_code)]
    MySql {
        host: String,
        port: u16,
        username: String,
        password: String,
        database_name: String,
    },
}

/// Represents the configuration options for various database types.
///
/// This enum defines supported database configurations, including `SQLite` (both on-disk
/// and in-memory) and provides a placeholder for `MySQL` support. It can be extended
/// further to accommodate other database management systems (DBMS) in the future.
///
/// # Variants
///
/// * `SqliteOnDisk`
///   Represents an ``SQLite`` database stored on disk, using a specific file path.
///
/// * `SqliteInMemory`
///   Represents an ``SQLite`` database that operates entirely in memory. This configuration
///   is typically used for testing or environments where persistence is not required.
///
/// * `MySql`
///   Provides a placeholder configuration for a ``MySQL`` database. This variant includes
///   parameters such as host, port, username, password, and the database name.
///
/// # Examples
///
/// ```rust,ignore
/// /// use std::path::PathBuf;
/// use crate::db::DatabaseConfig;
///
/// // Example: Creating an `SQLite` on-disk database configuration
/// let config = DatabaseConfig::SqliteOnDisk {
///     path: PathBuf::from("my_database.db"),
/// };
///
/// // Example: Creating an `SQLite` in-memory database configuration
/// let config_in_memory = DatabaseConfig::SqliteInMemory;
/// ```
pub enum DbConnection {
    Sqlite(SharedSqliteConnection),
    // Add other types for `MySQL`, PostgreSql, etc.
}

pub struct DatabaseManager {
    connection: DbConnection,
}

impl DatabaseManager {
    /// Creates a new `DatabaseManager` based on the provided configuration.
    pub fn new(config: &DatabaseConfig) -> Result<Self, WorklogError> {
        let connection = match config {
            DatabaseConfig::SqliteOnDisk { path } => Self::create_sqlite_connection(
                || Connection::open(path),
                || Cow::from(path.to_string_lossy().into_owned()),
            ),
            DatabaseConfig::SqliteInMemory => {
                Self::create_sqlite_connection(Connection::open_in_memory, || "in-memory".into())
            }
            DatabaseConfig::MySql { .. } => unimplemented!(),
        }?;

        // Initialize the schema (shared logic across all DB types).
        Self::initialize_schema(&connection).map_err(|e| {
            WorklogError::DatabaseError(format!("Failed to initialize schema: {e}"))
        })?;

        Ok(Self { connection })
    }

    /// Helper function to create a ``SQLite`` connection with error handling.
    fn create_sqlite_connection<F, G>(connect: F, context: G) -> Result<DbConnection, WorklogError>
    where
        F: FnOnce() -> rusqlite::Result<Connection>,
        G: FnOnce() -> std::borrow::Cow<'static, str>,
    {
        let connection = connect().map_err(|e| {
            WorklogError::DatabaseError(format!(
                "Failed to open `SQLite` database ({}): {}",
                context(),
                e
            ))
        })?;

        Ok(DbConnection::Sqlite(Arc::new(Mutex::new(connection))))
    }

    /// Internal method to handle schema initialization.
    fn initialize_schema(connection: &DbConnection) -> Result<(), WorklogError> {
        match connection {
            DbConnection::Sqlite(conn) => sqlite::create_schema(conn),
        }
    }

    /// Creates and returns an `Arc`-wrapped `SqliteIssueRepository` instance.
    ///
    /// This method uses the current database connection to initialize a new
    /// `SQLite`-based issue repository.
    ///
    /// # Returns
    ///
    /// An `Arc<SqliteIssueRepository>` instance, which can be shared across
    /// multiple components or threads safely.
    ///
    /// # Panics
    ///
    /// This method assumes the connection is valid and correctly initialized.
    /// If the connection is not properly configured, this function will
    /// panic or return unexpected behavior during repository operations.
    pub(crate) fn create_issue_repository(&self) -> Arc<SqliteIssueRepository> {
        match &self.connection {
            DbConnection::Sqlite(conn) => Arc::new(SqliteIssueRepository::new(conn.clone())),
        }
    }

    /// Creates and returns an `Arc`-wrapped `SqliteWorklogRepository` instance.
    ///
    /// This method uses the current database connection to initialize a new
    /// `SQLite`-based worklog repository.
    ///
    /// # Returns
    ///
    /// An `Arc<SqliteWorklogRepository>` instance, which can be shared across
    /// multiple components or threads safely.
    ///
    /// # Panics
    ///
    /// Similar to `create_issue_repository`, this method assumes the database
    /// connection is valid and correctly initialized. If the connection is
    /// improperly configured, it will result in a panic or undefined behavior
    /// during repository operations.
    pub(crate) fn create_worklog_repository(&self) -> Arc<SqliteWorklogRepository> {
        match &self.connection {
            DbConnection::Sqlite(conn) => Arc::new(SqliteWorklogRepository::new(conn.clone())),
        }
    }

    /// Creates and returns an `Arc`-wrapped `SqliteComponentRepository` instance.
    ///
    /// This method uses the current database connection to initialize a new
    /// `SQLite`-based component repository.
    ///
    /// # Returns
    ///
    /// An `Arc<SqliteComponentRepository>` instance, which can be shared across
    /// multiple components or threads safely.
    ///
    /// # Panics
    ///
    /// Similar to other repository creation methods, this function assumes
    /// the database connection is valid and correctly initialized. If the
    /// connection is improperly configured, it will result in a panic or
    /// undefined behavior during repository operations.
    pub(crate) fn create_user_repository(&self) -> Arc<dyn UserRepository> {
        match &self.connection {
            DbConnection::Sqlite(conn) => Arc::new(SqliteUserRepository::new(conn.clone())),
        }
    }

    /// Creates and returns an `Arc`-wrapped `SqliteComponentRepository` instance.
    ///
    /// This method utilizes the current database connection to initialize a new
    /// `SQLite`-based component repository, which provides operations related to
    /// specific components in the system.
    ///
    /// # Returns
    ///
    /// An `Arc<SqliteComponentRepository>` instance, enabling the component repository
    /// to be safely shared among threads and components that require access to
    /// component-related database operations.
    ///
    /// # Panics
    ///
    /// Similar to other repository creation methods, this function assumes
    /// that the database connection has been properly and validly initialized.
    /// If for any reason the connection is invalid or misconfigured, calling this
    /// function may result in a panic or undefined behavior during component repository
    /// operations.
    pub(crate) fn create_component_repository(&self) -> Arc<SqliteComponentRepository> {
        match &self.connection {
            DbConnection::Sqlite(conn) => Arc::new(SqliteComponentRepository::new(conn.clone())),
        }
    }
}
