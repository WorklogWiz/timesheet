use crate::error::WorklogError;
use jira::models::core::IssueKey;
use jira::models::project::Component;

pub trait ComponentRepository: Send + Sync {
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
        components: &Vec<Component>,
    ) -> Result<(), WorklogError>;
}
