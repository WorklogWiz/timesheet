use crate::error::WorklogError;
use crate::repository::database_manager::DbConnection;
use crate::repository::sqlite::tests::test_database_manager;
use crate::repository::sqlite::SharedSqliteConnection;

#[test]
fn test_foreign_keys_enabled() {
    // Create a test database manager
    let db_manager = test_database_manager().expect("Failed to create test database manager");

    // Get the connection (we know it's SQLite in test mode)
    let DbConnection::Sqlite(conn) = db_manager.get_connection();

    // Check if foreign keys are enabled
    let foreign_keys_enabled =
        is_foreign_keys_enabled(conn).expect("Failed to check foreign keys setting");

    assert!(foreign_keys_enabled, "Foreign keys should be enabled");
}

/// Helper function to check if foreign keys are enabled in an SQLite connection
fn is_foreign_keys_enabled(conn: &SharedSqliteConnection) -> Result<bool, WorklogError> {
    let conn = conn.lock().map_err(|_| WorklogError::LockPoisoned)?;
    let mut stmt = conn
        .prepare("PRAGMA foreign_keys")
        .map_err(|e| WorklogError::Sql(e.to_string()))?;

    let rows = stmt
        .query_map([], |row| row.get::<_, i32>(0))
        .map_err(|e| WorklogError::Sql(e.to_string()))?;

    // There should be exactly one row with value 1 if foreign keys are enabled
    for row in rows {
        return Ok(row.map_err(|e| WorklogError::Sql(e.to_string()))? == 1);
    }

    // If no rows returned (shouldn't happen, but just in case)
    Ok(false)
}
