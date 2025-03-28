use crate::error::WorklogError;
use crate::repository::component_repository::ComponentRepository;
use crate::repository::SharedSqliteConnection;
use jira::models::core::IssueKey;
use jira::models::project::Component;
use log::debug;
use rusqlite::params;

pub struct SqliteComponentRepository {
    connection: SharedSqliteConnection,
}

impl SqliteComponentRepository {
    pub(crate) fn new(connection: SharedSqliteConnection) -> Self {
        Self { connection }
    }
}

const CREATE_COMPONENT_TABLE_SQL: &str = r"
    CREATE TABLE IF NOT EXISTS component (
        id integer primary key not null,
        name varchar(1024) not null
    );
";

pub fn create_component_table(conn: &SharedSqliteConnection) -> Result<(), rusqlite::Error> {
    let conn = conn.lock().expect("component connection mutex poisoned");
    conn.execute(CREATE_COMPONENT_TABLE_SQL, [])?;
    Ok(())
}

const CREATE_ISSUE_COMPONENT_TABLE_SQL: &str = r"
    -- Association between the tables issue and component
    CREATE TABLE IF NOT EXISTS issue_component (
        id INTEGER PRIMARY KEY NOT NULL,
        key VARCHAR(32) NOT NULL,
        component_id INTEGER NOT NULL,
        FOREIGN KEY (key) REFERENCES issue(key) ON DELETE CASCADE,
        FOREIGN KEY (component_id) REFERENCES component(id) ON DELETE CASCADE,
        UNIQUE(key, component_id)
    );
";

pub fn create_issue_component_table(conn: &SharedSqliteConnection) -> Result<(), rusqlite::Error> {
    let conn = conn
        .lock()
        .expect("issue component connection mutex poisoned");
    conn.execute(CREATE_ISSUE_COMPONENT_TABLE_SQL, [])?;
    Ok(())
}

impl ComponentRepository for SqliteComponentRepository {
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
    fn create_component(
        &self,
        issue_key: &IssueKey,
        components: &[Component],
    ) -> Result<(), WorklogError> {
        debug!("Inserting components ...");

        let conn = self
            .connection
            .lock()
            .expect("component connection mutex poisoned");
        let mut insert_component_stmt = conn.prepare(
            "INSERT INTO component (id, name)
            VALUES (?1, ?2)
            ON CONFLICT(id) DO UPDATE SET name = excluded.name",
        )?;

        debug!(
            "Adding {} components for issue {issue_key}",
            components.len()
        );
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
        debug!("Adding issue_components, waiting for mutex");

        // Links the components with the issues to maintain the many-to-many relationship
        let mut insert_issue_component_stmt = conn
            .prepare("INSERT OR IGNORE INTO issue_component (key, component_id) VALUES (?1, ?2)")?;
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
