use crate::error::WorklogError;
use crate::storage::dbms::Dbms;
use jira::models::core::IssueKey;
use jira::models::project::Component;
use log::debug;
use rusqlite::params;

impl Dbms {
    ///
    /// Adds a list of components to the local database and associates them with the given issue key.
    ///
    /// This function inserts the provided `components` into the `component` table and ensures that
    /// they are linked with the specified `issue_key` in the `issue_component` table.
    /// If a component with the same ID already exists, it updates its name.
    /// The `issue_key` and component IDs are also added to the `issue_component` table, avoiding duplicates.
    ///
    /// # Arguments
    /// * `issue_key` - The issue key to associate the components with.
    /// * `components` - A list of `Component` objects to add to the database.
    ///
    /// # Errors
    /// Returns a `WorklogError` if any SQL operation fails during the insertion or association.
    ///
    /// # Panics
    /// This method panics if it encounters any error during the execution of the SQL statements.
    pub fn create_component(
        &self,
        issue_key: &IssueKey,
        components: &Vec<Component>,
    ) -> Result<(), WorklogError> {
        let mut insert_component_stmt = self.connection.prepare(
            "INSERT INTO component (id, name)
            VALUES (?1, ?2)
            ON CONFLICT(id) DO UPDATE SET name = excluded.name",
        )?;

        debug!("Adding components for issue {issue_key}");
        for component in components {
            debug!("Adding component id {} for issue {issue_key}", component.id);
            // Consider using the return value to count number of rows that were actually
            // inserted
            if let Err(e) =
                insert_component_stmt.execute(params![component.id, component.name.clone()])
            {
                panic!(
                    "Unable to insert component({},{}): {}",
                    component.id, component.name, e
                );
            }
        }
        // Links the components with the issues to maintain the many-to-many relationship
        let mut insert_issue_component_stmt = self.connection.prepare(
            "INSERT OR IGNORE INTO issue_component (issue_key, component_id) VALUES (?1, ?2)",
        )?;
        for component in components {
            debug!(
                "Adding issue_component ({}, {})",
                issue_key.value, component.id
            );
            if let Err(e) =
                insert_issue_component_stmt.execute(params![issue_key.value(), component.id])
            {
                panic!(
                    "Unable to insert issue_component({},{}): {}",
                    issue_key.value(),
                    component.id,
                    e
                );
            }
        }
        Ok(())
    }
}
