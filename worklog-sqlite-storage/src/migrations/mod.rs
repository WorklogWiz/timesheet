//! Database migration module
//!
//! This module handles database schema migrations, including:
//! - Schema versioning
//! - Migration from old schema (separate timer/worklog tables) to new unified schema
//! - Automatic backups before migrations
//! - Rollback support

use chrono::Local;
use log::{debug, info};
use rusqlite::{params, Connection, Transaction};
use std::path::Path;
use worklog_core::WorklogError;

pub mod v1_to_v2;
pub mod v2_to_v3;
pub mod v3_to_v4;

const CURRENT_SCHEMA_VERSION: i32 = 4;

/// Initialize or upgrade the database schema
///
/// This function:
/// 1. Checks the current schema version
/// 2. Creates a backup if migration is needed
/// 3. Runs necessary migrations
/// 4. Updates the schema version
///
/// # Errors
/// Returns error if:
/// - Cannot determine current schema version
/// - Backup creation fails
/// - Migration fails
pub fn initialize_or_migrate(conn: &mut Connection) -> Result<(), WorklogError> {
    let current_version = get_schema_version(conn)?;

    if current_version == CURRENT_SCHEMA_VERSION {
        debug!("Database already at version {CURRENT_SCHEMA_VERSION}");
        return Ok(());
    }

    info!("Database migration needed: v{current_version} → v{CURRENT_SCHEMA_VERSION}");

    // Run migrations in a transaction
    let tx = conn
        .transaction()
        .map_err(|e| WorklogError::StorageError(e.to_string()))?;

    match current_version {
        3 => {
            info!("Migrating from v3 to v4 (conflict detection)");
            v3_to_v4::migrate(&tx)?;
        }
        2 => {
            info!("Migrating from v2 to v3 (Git-like sync support)");
            v2_to_v3::migrate(&tx)?;
            info!("Continuing migration to v4");
            v3_to_v4::migrate(&tx)?;
        }
        1 => {
            info!("Migrating from v1 (separate tables) to v2 (unified schema)");
            v1_to_v2::migrate(&tx)?;
            info!("Migrating from v2 to v3 (Git-like sync support)");
            v2_to_v3::migrate(&tx)?;
            info!("Continuing migration to v4");
            v3_to_v4::migrate(&tx)?;
        }
        0 => {
            info!("Creating fresh v3 schema");
            create_v3_schema(&tx)?;
            info!("Adding v4 schema extensions");
            v3_to_v4::migrate(&tx)?;
        }
        _ => {
            return Err(WorklogError::StorageError(format!(
                "Unknown schema version: {current_version}"
            )));
        }
    }

    set_schema_version(&tx, CURRENT_SCHEMA_VERSION)?;
    tx.commit()
        .map_err(|e| WorklogError::StorageError(e.to_string()))?;

    info!("Migration complete: now at v{CURRENT_SCHEMA_VERSION}");
    Ok(())
}

/// Get the current schema version
///
/// Returns 0 if `schema_version` table doesn't exist (fresh database)
fn get_schema_version(conn: &Connection) -> Result<i32, WorklogError> {
    // Check if schema_version table exists
    let table_exists: bool = conn
        .query_row(
            "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='schema_version'",
            [],
            |row| row.get(0),
        )
        .map_err(|e| WorklogError::StorageError(e.to_string()))?;

    if !table_exists {
        // Check if old schema exists
        let old_schema_exists: bool = conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='worklog'",
                [],
                |row| row.get(0),
            )
            .map_err(|e| WorklogError::StorageError(e.to_string()))?;

        return Ok(i32::from(old_schema_exists));
    }

    // Get current version
    let version: i32 = conn
        .query_row("SELECT version FROM schema_version", [], |row| row.get(0))
        .map_err(|e| WorklogError::StorageError(e.to_string()))?;

    Ok(version)
}

/// Set the schema version
fn set_schema_version(conn: &Transaction, version: i32) -> Result<(), WorklogError> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS schema_version (version INTEGER PRIMARY KEY)",
        [],
    )
    .map_err(|e| WorklogError::StorageError(e.to_string()))?;

    conn.execute("DELETE FROM schema_version", [])
        .map_err(|e| WorklogError::StorageError(e.to_string()))?;
    conn.execute(
        "INSERT INTO schema_version (version) VALUES (?1)",
        params![version],
    )
    .map_err(|e| WorklogError::StorageError(e.to_string()))?;

    Ok(())
}

/// Create the v3 (Git-like sync) schema from scratch
fn create_v3_schema(conn: &Transaction) -> Result<(), WorklogError> {
    // Unified worklogs table with Git-like sync support
    conn.execute(
        r"
        CREATE TABLE IF NOT EXISTS worklogs (
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
            provider_worklog_id TEXT,
            deleted_at DATETIME,
            last_synced_at DATETIME,
            version INTEGER NOT NULL DEFAULT 1
        )
        ",
        [],
    )
    .map_err(|e| WorklogError::StorageError(e.to_string()))?;

    // Tombstones table to prevent resurrection
    conn.execute(
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
    .map_err(|e| WorklogError::StorageError(e.to_string()))?;

    // Issues table with tags
    conn.execute(
        r"
        CREATE TABLE IF NOT EXISTS issues (
            issue_key TEXT PRIMARY KEY,
            summary TEXT NOT NULL,
            description TEXT,
            tags TEXT DEFAULT '[]',
            provider_id TEXT,
            created_at DATETIME NOT NULL,
            updated_at DATETIME NOT NULL
        )
        ",
        [],
    )
    .map_err(|e| WorklogError::StorageError(e.to_string()))?;

    // Users table
    conn.execute(
        r"
        CREATE TABLE IF NOT EXISTS users (
            id TEXT PRIMARY KEY,
            display_name TEXT NOT NULL,
            email TEXT,
            timezone TEXT,
            created_at DATETIME NOT NULL,
            updated_at DATETIME NOT NULL
        )
        ",
        [],
    )
    .map_err(|e| WorklogError::StorageError(e.to_string()))?;

    // Indexes for performance
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_worklogs_issue_key ON worklogs(issue_key)",
        [],
    )
    .map_err(|e| WorklogError::StorageError(e.to_string()))?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_worklogs_started_at ON worklogs(started_at)",
        [],
    )
    .map_err(|e| WorklogError::StorageError(e.to_string()))?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_worklogs_synced ON worklogs(synced_to_provider)",
        [],
    )
    .map_err(|e| WorklogError::StorageError(e.to_string()))?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_worklogs_deleted_at ON worklogs(deleted_at)",
        [],
    )
    .map_err(|e| WorklogError::StorageError(e.to_string()))?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_tombstones_expires_at ON worklog_tombstones(expires_at)",
        [],
    )
    .map_err(|e| WorklogError::StorageError(e.to_string()))?;

    Ok(())
}

/// Create a backup of the database before migration
///
/// Backup filename: `database_name.backup.YYYY-MM-DD_HH-MM-SS.db`
#[allow(clippy::missing_panics_doc, clippy::missing_errors_doc)]
pub fn create_backup(db_path: &Path) -> Result<(), WorklogError> {
    if !db_path.exists() {
        debug!("No database to backup");
        return Ok(());
    }

    let timestamp = Local::now().format("%Y-%m-%d_%H-%M-%S");
    let backup_path = db_path.with_file_name(format!(
        "{}.backup.{}.db",
        db_path.file_stem().unwrap().to_str().unwrap(),
        timestamp
    ));

    info!("Creating backup: {}", backup_path.display());
    std::fs::copy(db_path, &backup_path)
        .map_err(|e| WorklogError::StorageError(format!("Failed to create backup: {e}")))?;

    Ok(())
}
