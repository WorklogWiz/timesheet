//! Migration from schema v2 to v3
//!
//! This migration adds Git-like sync support:
//! - Soft delete support (`deleted_at` column)
//! - Sync tracking (`last_synced_at`, `version` columns)
//! - Tombstones table to prevent resurrection of deleted items

use log::info;
use rusqlite::Transaction;
use worklog_core::WorklogError;

/// Migrate from v2 to v3 schema
///
/// # Changes
/// - Add `deleted_at` column to worklogs (soft delete)
/// - Add `last_synced_at` column to worklogs (track sync state)
/// - Add `version` column to worklogs (optimistic locking)
/// - Create `worklog_tombstones` table (prevent resurrection)
///
/// # Errors
/// Returns error if any SQL statement fails
pub fn migrate(tx: &Transaction) -> Result<(), WorklogError> {
    info!("Starting v2 → v3 migration (Git-like sync support)");

    // 1. Add deleted_at column for soft deletes
    info!("  Adding deleted_at column");
    tx.execute("ALTER TABLE worklogs ADD COLUMN deleted_at DATETIME", [])
        .map_err(|e| WorklogError::StorageError(format!("Failed to add deleted_at column: {e}")))?;

    // 2. Add last_synced_at column to track sync state
    info!("  Adding last_synced_at column");
    tx.execute(
        "ALTER TABLE worklogs ADD COLUMN last_synced_at DATETIME",
        [],
    )
    .map_err(|e| WorklogError::StorageError(format!("Failed to add last_synced_at column: {e}")))?;

    // 3. Add version column for optimistic locking (default 1)
    info!("  Adding version column");
    tx.execute(
        "ALTER TABLE worklogs ADD COLUMN version INTEGER NOT NULL DEFAULT 1",
        [],
    )
    .map_err(|e| WorklogError::StorageError(format!("Failed to add version column: {e}")))?;

    // 4. Create tombstones table to prevent deleted items from resurrecting
    info!("  Creating worklog_tombstones table");
    tx.execute(
        r"
        CREATE TABLE IF NOT EXISTS worklog_tombstones (
            provider_worklog_id TEXT PRIMARY KEY,
            deleted_at DATETIME NOT NULL,
            tracker_name TEXT NOT NULL DEFAULT 'jira',
            expires_at DATETIME NOT NULL
        )
        ",
        [],
    )
    .map_err(|e| WorklogError::StorageError(format!("Failed to create tombstones table: {e}")))?;

    // 5. Create index on deleted_at for efficient filtering
    info!("  Creating index on deleted_at");
    tx.execute(
        "CREATE INDEX IF NOT EXISTS idx_worklogs_deleted_at ON worklogs(deleted_at)",
        [],
    )
    .map_err(|e| WorklogError::StorageError(format!("Failed to create deleted_at index: {e}")))?;

    // 6. Create index on tombstones expires_at for cleanup
    info!("  Creating index on tombstones expires_at");
    tx.execute(
        "CREATE INDEX IF NOT EXISTS idx_tombstones_expires_at ON worklog_tombstones(expires_at)",
        [],
    )
    .map_err(|e| WorklogError::StorageError(format!("Failed to create expires_at index: {e}")))?;

    // 7. For existing synced worklogs, set last_synced_at = updated_at
    // This gives us a baseline for conflict detection
    info!("  Initializing last_synced_at for existing synced entries");
    tx.execute(
        "UPDATE worklogs SET last_synced_at = updated_at WHERE synced_to_provider = 1",
        [],
    )
    .map_err(|e| WorklogError::StorageError(format!("Failed to initialize last_synced_at: {e}")))?;

    info!("✅ v2 → v3 migration complete");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    #[test]
    fn test_v2_to_v3_migration() {
        let mut conn = Connection::open_in_memory().unwrap();
        let tx = conn.transaction().unwrap();

        // Create v2 schema
        tx.execute(
            r"
            CREATE TABLE worklogs (
                id TEXT PRIMARY KEY,
                issue_key TEXT,
                started_at DATETIME NOT NULL,
                stopped_at DATETIME,
                comment TEXT,
                tags TEXT DEFAULT '[]',
                time_spent_seconds INTEGER,
                synced_to_provider BOOLEAN NOT NULL DEFAULT 0,
                created_at DATETIME NOT NULL,
                updated_at DATETIME NOT NULL,
                provider_worklog_id TEXT
            )
            ",
            [],
        )
        .unwrap();

        // Insert test data
        tx.execute(
            r"
            INSERT INTO worklogs (
                id, issue_key, started_at, stopped_at,
                synced_to_provider, created_at, updated_at, provider_worklog_id
            ) VALUES (
                'test-1', 'BTS-1', '2024-01-01 10:00:00', '2024-01-01 11:00:00',
                1, '2024-01-01 10:00:00', '2024-01-01 11:00:00', 'jira-123'
            )
            ",
            [],
        )
        .unwrap();

        // Run migration
        migrate(&tx).unwrap();

        // Verify new columns exist
        let deleted_at: Option<String> = tx
            .query_row(
                "SELECT deleted_at FROM worklogs WHERE id = 'test-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(deleted_at.is_none());

        let last_synced_at: Option<String> = tx
            .query_row(
                "SELECT last_synced_at FROM worklogs WHERE id = 'test-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(last_synced_at.is_some()); // Should be set to updated_at

        let version: i32 = tx
            .query_row(
                "SELECT version FROM worklogs WHERE id = 'test-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(version, 1);

        // Verify tombstones table exists
        let table_exists: bool = tx
            .query_row(
                "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='worklog_tombstones'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(table_exists);

        tx.commit().unwrap();
    }
}
