use crate::error::WorklogError;
use crate::repository::sqlite;
use crate::repository::sqlite::sqlite_component_repo::SqliteComponentRepository;
use crate::repository::sqlite::sqlite_issue_repo::SqliteIssueRepository;
use crate::repository::sqlite::sqlite_user_repo::SqliteUserRepository;
use crate::repository::sqlite::sqlite_worklog_repo::SqliteWorklogRepository;
use rusqlite::{Connection, Result};
use std::path::PathBuf;
use std::sync::Arc;

/// Represents parameters for initializing the database connection
pub enum DatabaseConfig {
    /// SQLite database with a specific file path
    SqliteOnDisk { path: PathBuf },

    /// SQLite database that runs entirely in memory
    SqliteInMemory,

    /// A placeholder for a MySQL database (extendable to support other DBMS types)
    MySql {
        host: String,
        port: u16,
        username: String,
        password: String,
        database_name: String,
    },
}

pub struct DatabaseManager {
    connection: Arc<Connection>,
}

impl DatabaseManager {
    /// Creates a new `DatabaseManager` based on the provided configuration.
    pub fn new(config: &DatabaseConfig) -> Result<Self, WorklogError> {
        let connection = match config {
            // SQLite (on-disk)
            DatabaseConfig::SqliteOnDisk { path } => Connection::open(path)?,

            // SQLite (in-memory)
            DatabaseConfig::SqliteInMemory => Connection::open_in_memory()?,

            // MySQL (support is not implemented here but serves as an example)
            DatabaseConfig::MySql { .. } => {
                // Placeholder: Your MySQL connection logic would go here
                unimplemented!("MySQL support is not yet implemented.")
            }
        };

        let connection = Arc::new(connection);

        // Initialize the schema (shared logic across all DB types)
        Self::initialize_schema(connection.clone(), config)?;

        Ok(Self { connection })
    }

    /// Internal method to handle schema initialization.
    fn initialize_schema(
        connection: Arc<Connection>,
        config: &DatabaseConfig,
    ) -> Result<(), WorklogError> {
        match config {
            DatabaseConfig::SqliteOnDisk { .. } | DatabaseConfig::SqliteInMemory => {
                sqlite::create_schema(connection.clone())
            }
            DatabaseConfig::MySql { .. } => {
                unimplemented!("MySQL support is not yet implemented.")
            }
        }
    }

    /// Provide access to the shared database connection.
    pub(crate) fn get_connection(&self) -> Arc<Connection> {
        self.connection.clone()
    }

    pub(crate) fn create_issue_repository(&self) -> Arc<SqliteIssueRepository> {
        Arc::new(SqliteIssueRepository::new(self.get_connection()))
    }

    pub(crate) fn create_worklog_repository(&self) -> Arc<SqliteWorklogRepository> {
        Arc::new(SqliteWorklogRepository::new(self.get_connection()))
    }

    pub(crate) fn create_user_repository(&self) -> Arc<SqliteUserRepository> {
        Arc::new(SqliteUserRepository::new(self.get_connection()))
    }

    pub(crate) fn create_component_repository(&self) -> Arc<SqliteComponentRepository> {
        Arc::new(SqliteComponentRepository::new(self.get_connection()))
    }
}
