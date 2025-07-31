mod schema_tests;

use super::*;
use crate::repository::database_manager::{DatabaseConfig, DatabaseManager};

/// Creates a `DatabaseManager` with an in-memory database suitable for testing.
pub fn test_database_manager() -> Result<DatabaseManager, WorklogError> {
    DatabaseManager::new(&DatabaseConfig::SqliteInMemory)
}
