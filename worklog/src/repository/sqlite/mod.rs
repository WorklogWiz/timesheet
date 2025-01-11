use crate::error::WorklogError;
use rusqlite::Connection;
use std::fs;
use std::path::Path;
use std::sync::Arc;

pub(crate) mod sqlite_user_repo;
pub(crate) mod sqlite_worklog_repo;

pub(crate) mod sqlite_component_repo;
pub(crate) mod sqlite_issue_repo;

/// Creates the entire database schema by running schema creation functions for all entities.
#[allow(clippy::module_name_repetitions)]
pub(crate) fn create_schema(connection: Arc<Connection>) -> Result<(), WorklogError> {
    sqlite_issue_repo::create_issue_table(connection.clone())?;
    sqlite_worklog_repo::create_worklog_table(connection.clone())?;
    sqlite_component_repo::create_component_table(connection.clone())?;
    sqlite_user_repo::create_schema(connection.clone())?;
    Ok(())
}

pub(crate) fn create_connection(dbms_path: &Path) -> Result<rusqlite::Connection, WorklogError> {
    if let Some(parent) = dbms_path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)?;
        }
    }
    let connection = Connection::open(dbms_path)?;
    Ok(connection)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::repository::database_manager::{DatabaseConfig, DatabaseManager};
    use crate::repository::sqlite::sqlite_issue_repo::SqliteIssueRepository;
    use crate::repository::sqlite::sqlite_worklog_repo::SqliteWorklogRepository;

    pub fn setup() -> Result<Arc<Connection>, WorklogError> {
        Ok(DatabaseManager::new(&DatabaseConfig::SqliteInMemory)?.get_connection())
    }

    pub fn create_issue_repo_for_test() -> Arc<SqliteIssueRepository> {
        DatabaseManager::new(&DatabaseConfig::SqliteInMemory)
            .unwrap()
            .create_issue_repository()
    }

    pub fn create_worklog_repo_for_test() -> Arc<SqliteWorklogRepository> {
        DatabaseManager::new(&DatabaseConfig::SqliteInMemory)
            .unwrap()
            .create_worklog_repository()
    }
}
