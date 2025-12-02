//! Migration from v3 to v4: Add conflict detection field
//!
//! Changes:
//! - Add `has_conflict` BOOLEAN field to worklogs table

use log::info;
use rusqlite::Transaction;
use worklog_core::WorklogError;

/// Migrate from v3 to v4 schema
///
/// Adds `has_conflict` field for tracking sync conflicts
///
/// # Errors
/// Returns error if SQL operations fail
pub fn migrate(tx: &Transaction) -> Result<(), WorklogError> {
    info!("Adding has_conflict field to worklogs table");

    // Add has_conflict column (default FALSE for existing entries)
    tx.execute(
        "ALTER TABLE worklogs ADD COLUMN has_conflict INTEGER NOT NULL DEFAULT 0",
        [],
    )
    .map_err(|e| WorklogError::StorageError(format!("Failed to add has_conflict column: {e}")))?;

    info!("Migration v3 → v4 complete");
    Ok(())
}
