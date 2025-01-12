use crate::error::WorklogError;
use rusqlite::Connection;
use std::sync::{Arc, Mutex};

pub mod sqlite_user_repo;
pub mod sqlite_worklog_repo;

pub mod sqlite_component_repo;
pub mod sqlite_issue_repo;

/// Creates the entire database schema by running schema creation functions for all entities.
#[allow(clippy::module_name_repetitions)]
pub(crate) fn create_schema(connection: Arc<Mutex<Connection>>) -> Result<(), WorklogError> {
    sqlite_issue_repo::create_issue_table(connection.clone())?;
    sqlite_worklog_repo::create_worklog_table(connection.clone())?;
    sqlite_component_repo::create_component_table(connection.clone())?;
    sqlite_component_repo::create_issue_component_table(connection.clone())?;
    sqlite_user_repo::create_schema(connection.clone())?;
    Ok(())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::repository::database_manager::{DatabaseConfig, DatabaseManager};

    pub fn test_database_manager() -> Result<DatabaseManager, WorklogError> {
        Ok(DatabaseManager::new(&DatabaseConfig::SqliteInMemory)?)
    }
}
