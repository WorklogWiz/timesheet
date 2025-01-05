
pub mod user;
pub mod worklog;
pub mod issue;
pub mod component;
pub mod issue_component;

use rusqlite::Connection;

// Re-export individual entity schema modules

use user::create_user_table;
use worklog::create_worklog_table;
use crate::error::WorklogError;
use crate::storage::schema::component::create_component_table;
use crate::storage::schema::issue::create_issue_table;
use crate::storage::schema::issue_component::create_issue_component_table;

/// Creates the entire database schema by running schema creation functions for all entities.
pub fn create_schema(connection: &Connection) -> Result<(),WorklogError> {
    create_issue_table(connection).map_err(|e| WorklogError::Sql(format!("Unable to create table 'issue': {e}")))?;
    create_worklog_table(connection).map_err(|e| WorklogError::Sql(format!("Unable to create table 'worklog': {e}")))?;
    create_component_table(connection).map_err(|e| WorklogError::Sql(format!("Unable to create table 'component': {e}")))?;
    create_issue_component_table(connection).map_err(|e| WorklogError::Sql(format!("Unable to create table 'issue_component': {e}")))?;
    create_user_table(connection).map_err(|e| WorklogError::Sql(format!("Unable to create table 'user': {e}")))?;
    
    Ok(())
}
