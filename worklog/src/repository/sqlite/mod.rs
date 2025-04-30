use crate::error::WorklogError;
use rusqlite::Connection;
use std::sync::{Arc, Mutex};

pub(crate) mod sqlite_component_repo;
pub(crate) mod sqlite_issue_repo;
pub(crate) mod sqlite_timer_repo;
pub(crate) mod sqlite_user_repo;
pub(crate) mod sqlite_worklog_repo;

/// A thread-safe, shared connection to an ``SQLite`` database,
pub(crate) type SharedSqliteConnection = Arc<Mutex<Connection>>;

/// Creates the entire database schema by running schema creation functions for all entities.
#[allow(clippy::module_name_repetitions)]
pub(crate) fn create_schema(connection: &SharedSqliteConnection) -> Result<(), WorklogError> {
    sqlite_issue_repo::create_issue_table(&connection.clone())?;
    sqlite_worklog_repo::create_worklog_table(&connection.clone())?;
    sqlite_timer_repo::create_timer_table(&connection.clone())?;
    sqlite_component_repo::create_component_table(&connection.clone())?;
    // many-to-many relationship between issues and components
    sqlite_component_repo::create_issue_component_table(&connection.clone())?;
    sqlite_user_repo::create_schema(&connection.clone())?;
    Ok(())
}

#[cfg(test)]
mod tests;
