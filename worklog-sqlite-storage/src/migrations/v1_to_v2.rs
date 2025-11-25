//! Migration from v1 (separate tables) to v2 (unified schema) -  FIXED VERSION
//!
//! This migration:
//! - Merges `timer` and `worklog` tables into unified `worklogs` table
//! - Converts `component` + `issue_component` → JSON `tags` in `issues` table
//! - Preserves all data
//! - Is atomic (all-or-nothing)

use chrono::Local;
use log::{debug, info};
use rusqlite::{params, OptionalExtension, Transaction};
use worklog_core::WorklogError;

/// Migrate from v1 to v2 schema
pub fn migrate(tx: &Transaction) -> Result<(), WorklogError> {
    info!("Starting v1 → v2 migration");

    // Create new v2 schema
    create_v2_tables(tx)?;

    // Migrate data
    migrate_users(tx)?;
    migrate_issues_with_tags(tx)?;
    migrate_worklogs(tx)?;
    migrate_timers(tx)?;

    info!("V1 → v2 migration complete");
    Ok(())
}

fn create_v2_tables(tx: &Transaction) -> Result<(), WorklogError> {
    debug!("Creating v2 schema tables");

    // Unified worklogs table
    tx.execute(
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
            provider_worklog_id TEXT
        )
        ",
        [],
    )
    .map_err(|e| WorklogError::StorageError(e.to_string()))?;

    // Issues table with tags
    tx.execute(
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

    // Users table (structure unchanged)
    tx.execute(
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

    // Indexes
    tx.execute(
        "CREATE INDEX IF NOT EXISTS idx_worklogs_issue_key ON worklogs(issue_key)",
        [],
    )
    .map_err(|e| WorklogError::StorageError(e.to_string()))?;
    tx.execute(
        "CREATE INDEX IF NOT EXISTS idx_worklogs_started_at ON worklogs(started_at)",
        [],
    )
    .map_err(|e| WorklogError::StorageError(e.to_string()))?;
    tx.execute(
        "CREATE INDEX IF NOT EXISTS idx_worklogs_synced ON worklogs(synced_to_provider)",
        [],
    )
    .map_err(|e| WorklogError::StorageError(e.to_string()))?;

    Ok(())
}

fn migrate_users(tx: &Transaction) -> Result<(), WorklogError> {
    debug!("Migrating users");

    let old_table_exists: bool = tx
        .query_row(
            "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='user'",
            [],
            |row| row.get(0),
        )
        .map_err(|e| WorklogError::StorageError(e.to_string()))?;

    if !old_table_exists {
        debug!("No old user table found");
        return Ok(());
    }

    let now = Local::now().to_rfc3339();

    // Check if old schema has 'id' or 'account_id' as primary key
    let has_id_column: bool = tx
        .query_row(
            "SELECT COUNT(*) > 0 FROM pragma_table_info('user') WHERE name='id'",
            [],
            |row| row.get(0),
        )
        .unwrap_or(false);

    let id_column = if has_id_column { "id" } else { "account_id" };

    // Check if old schema has 'timezone' column
    let has_timezone: bool = tx
        .query_row(
            "SELECT COUNT(*) > 0 FROM pragma_table_info('user') WHERE name='timezone'",
            [],
            |row| row.get(0),
        )
        .unwrap_or(false);

    let timezone_expr = if has_timezone {
        "COALESCE(timezone, 'UTC')"
    } else {
        "'UTC'"
    };

    tx.execute(
        &format!(
            r"
        INSERT INTO users (id, display_name, email, timezone, created_at, updated_at)
        SELECT {id_column}, display_name, COALESCE(email, ''), {timezone_expr}, ?1, ?1
        FROM user
        "
        ),
        params![now],
    )
    .map_err(|e| WorklogError::StorageError(e.to_string()))?;

    info!("Users migrated");
    Ok(())
}

fn migrate_issues_with_tags(tx: &Transaction) -> Result<(), WorklogError> {
    debug!("Migrating issues with tags");

    let old_table_exists: bool = tx
        .query_row(
            "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='issue'",
            [],
            |row| row.get(0),
        )
        .map_err(|e| WorklogError::StorageError(e.to_string()))?;

    if !old_table_exists {
        debug!("No old issue table found");
        return Ok(());
    }

    let now = Local::now().to_rfc3339();

    // Check if old schema has 'issue_key' or 'key' column
    let has_issue_key: bool = tx
        .query_row(
            "SELECT COUNT(*) > 0 FROM pragma_table_info('issue') WHERE name='issue_key'",
            [],
            |row| row.get(0),
        )
        .unwrap_or(false);

    let key_column = if has_issue_key { "issue_key" } else { "key" };

    // For each issue, collect its components and convert to JSON tags
    let mut stmt = tx
        .prepare(&format!(
            r"
        SELECT i.{key_column}, i.summary
        FROM issue i
        "
        ))
        .map_err(|e| WorklogError::StorageError(e.to_string()))?;

    let issues: Vec<(String, String)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .map_err(|e| WorklogError::StorageError(e.to_string()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| WorklogError::StorageError(e.to_string()))?;

    drop(stmt); // Release the borrow

    for (issue_key, summary) in issues {
        // Get components for this issue
        let tags = get_components_for_issue(tx, &issue_key)?;
        let tags_json = serde_json::to_string(&tags).unwrap_or_else(|_| "[]".to_string());

        tx.execute(
            r"
            INSERT INTO issues (issue_key, summary, description, tags, provider_id, created_at, updated_at)
            VALUES (?1, ?2, NULL, ?3, NULL, ?4, ?4)
            ",
            params![issue_key, summary, tags_json, now],
        )
        .map_err(|e| WorklogError::StorageError(e.to_string()))?;
    }

    info!("Issues migrated with tags");
    Ok(())
}

fn get_components_for_issue(
    tx: &Transaction,
    issue_key: &str,
) -> Result<Vec<String>, WorklogError> {
    let component_table_exists: bool = tx
        .query_row(
            "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='component'",
            [],
            |row| row.get(0),
        )
        .map_err(|e| WorklogError::StorageError(e.to_string()))?;

    if !component_table_exists {
        return Ok(vec![]);
    }

    let issue_component_exists: bool = tx
        .query_row(
            "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='issue_component'",
            [],
            |row| row.get(0),
        )
        .map_err(|e| WorklogError::StorageError(e.to_string()))?;

    if !issue_component_exists {
        return Ok(vec![]);
    }

    // Get the issue ID first (check for 'key' or 'issue_key' column)
    let has_issue_key: bool = tx
        .query_row(
            "SELECT COUNT(*) > 0 FROM pragma_table_info('issue') WHERE name='issue_key'",
            [],
            |row| row.get(0),
        )
        .unwrap_or(false);

    let key_column = if has_issue_key { "issue_key" } else { "key" };

    let issue_id: Option<i64> = tx
        .query_row(
            &format!("SELECT id FROM issue WHERE {key_column} = ?1"),
            params![issue_key],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| WorklogError::StorageError(e.to_string()))?;

    let Some(issue_id) = issue_id else {
        return Ok(vec![]);
    };

    // Get components
    // Check if issue_component has 'issue_id' or 'key' column
    let has_issue_id_col: bool = tx
        .query_row(
            "SELECT COUNT(*) > 0 FROM pragma_table_info('issue_component') WHERE name='issue_id'",
            [],
            |row| row.get(0),
        )
        .unwrap_or(false);

    if has_issue_id_col {
        // Newer schema with issue_id foreign key
        let mut stmt = tx
            .prepare(
                r"
            SELECT c.name
            FROM component c
            JOIN issue_component ic ON c.id = ic.component_id
            WHERE ic.issue_id = ?1
            ",
            )
            .map_err(|e| WorklogError::StorageError(e.to_string()))?;

        let tags: Vec<String> = stmt
            .query_map(params![issue_id], |row| row.get(0))
            .map_err(|e| WorklogError::StorageError(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| WorklogError::StorageError(e.to_string()))?;

        Ok(tags)
    } else {
        // Older schema with issue key directly in issue_component
        let mut stmt = tx
            .prepare(
                r"
            SELECT c.name
            FROM component c
            JOIN issue_component ic ON c.id = ic.component_id
            WHERE ic.key = ?1
            ",
            )
            .map_err(|e| WorklogError::StorageError(e.to_string()))?;

        let tags: Vec<String> = stmt
            .query_map(params![issue_key], |row| row.get(0))
            .map_err(|e| WorklogError::StorageError(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| WorklogError::StorageError(e.to_string()))?;

        Ok(tags)
    }
}

fn migrate_worklogs(tx: &Transaction) -> Result<(), WorklogError> {
    debug!("Migrating worklogs");

    let old_table_exists: bool = tx
        .query_row(
            "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='worklog'",
            [],
            |row| row.get(0),
        )
        .map_err(|e| WorklogError::StorageError(e.to_string()))?;

    if !old_table_exists {
        debug!("No old worklog table found");
        return Ok(());
    }

    let now = Local::now().to_rfc3339();

    // Check if old schema has jira_worklog_id column (separate from id)
    let has_jira_id_column: bool = tx
        .query_row(
            "SELECT COUNT(*) > 0 FROM pragma_table_info('worklog') WHERE name='jira_worklog_id'",
            [],
            |row| row.get(0),
        )
        .unwrap_or(false);

    let (new_id_expr, provider_id_expr) = if has_jira_id_column {
        // Newer v1 schema: has separate jira_worklog_id column
        // Use id for new id, jira_worklog_id for provider_worklog_id
        ("'worklog_' || CAST(id AS TEXT)", "jira_worklog_id")
    } else {
        // Older v1 schema: id IS the Jira worklog ID
        // Generate new UUID for id, use old id for provider_worklog_id
        (
            "lower(hex(randomblob(4)) || '-' || hex(randomblob(2)) || '-' || hex(randomblob(2)) || '-' || hex(randomblob(2)) || '-' || hex(randomblob(6)))",
            "CAST(id AS TEXT)"
        )
    };

    tx.execute(
        &format!(
            r"
        INSERT INTO worklogs (
            id, issue_key, started_at, stopped_at, comment,
            tags, time_spent_seconds, synced_to_provider,
            created_at, updated_at, provider_worklog_id
        )
        SELECT
            {new_id_expr},
            issue_key,
            started,
            datetime(started, '+' || time_spent_seconds || ' seconds'),
            comment,
            '[]',
            time_spent_seconds,
            1,
            ?1,
            ?1,
            {provider_id_expr}
        FROM worklog
        "
        ),
        params![now],
    )
    .map_err(|e| WorklogError::StorageError(e.to_string()))?;

    info!("Worklogs migrated");
    Ok(())
}

fn migrate_timers(tx: &Transaction) -> Result<(), WorklogError> {
    debug!("Migrating timers");

    let old_table_exists: bool = tx
        .query_row(
            "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='timer'",
            [],
            |row| row.get(0),
        )
        .map_err(|e| WorklogError::StorageError(e.to_string()))?;

    if !old_table_exists {
        debug!("No old timer table found");
        return Ok(());
    }

    let now = Local::now().to_rfc3339();

    tx.execute(
        r"
        INSERT INTO worklogs (
            id, issue_key, started_at, stopped_at, comment,
            tags, time_spent_seconds, synced_to_provider,
            created_at, updated_at, provider_worklog_id
        )
        SELECT
            'timer_' || CAST(id AS TEXT),
            issue_key,
            started,
            NULL,
            comment,
            '[]',
            NULL,
            0,
            ?1,
            ?1,
            NULL
        FROM timer
        ",
        params![now],
    )
    .map_err(|e| WorklogError::StorageError(e.to_string()))?;

    info!("Timers migrated");
    Ok(())
}
