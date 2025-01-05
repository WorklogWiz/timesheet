
pub mod user;
pub mod worklog;
pub mod issue;
pub mod component;
pub mod issue_component;

use rusqlite::Connection;
use anyhow::{Result, Context};

// Re-export individual entity schema modules

use user::create_user_table;
use worklog::create_worklog_table;
use crate::error::WorklogError;
use crate::storage::schema::component::create_component_table;
use crate::storage::schema::issue::create_issue_table;
use crate::storage::schema::issue_component::create_issue_component_table;

/// Creates the entire database schema by running schema creation functions for all entities.
pub fn create_schema(connection: &Connection) -> Result<(),WorklogError> {
    create_issue_table(connection).map_err("Failed to create `issue` table")?;
    create_worklog_table(connection).context("Failed to create `worklog` table")?;
    create_component_table(connection).context("Failed to create `component` table")?;
    create_issue_component_table(connection).context("Failed to create `issue_component` table")?;
    create_user_table(connection).context("Failed to create `user` table")?;
    
    Ok(())
}