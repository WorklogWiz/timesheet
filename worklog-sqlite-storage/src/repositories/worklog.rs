use async_trait::async_trait;
use chrono::{DateTime, Local};
use rusqlite::{params, Connection, OptionalExtension};
use std::sync::{Arc, Mutex};
use worklog_core::{WorklogEntry, WorklogResult};

pub(crate) struct WorklogRepository {
    connection: Arc<Mutex<Connection>>,
}

#[allow(clippy::needless_pass_by_value)]
impl WorklogRepository {
    pub(crate) fn new(connection: Arc<Mutex<Connection>>) -> Self {
        Self { connection }
    }

    /// Helper to convert database errors to `WorklogError`
    fn to_worklog_error(e: rusqlite::Error) -> worklog_core::WorklogError {
        worklog_core::WorklogError::StorageError(e.to_string())
    }

    /// Helper to handle lock errors
    fn lock_error() -> worklog_core::WorklogError {
        worklog_core::WorklogError::StorageError("Database lock poisoned".to_string())
    }

    /// Helper to map a database row to a `WorklogEntry`
    fn map_row(row: &rusqlite::Row) -> rusqlite::Result<WorklogEntry> {
        Ok(WorklogEntry {
            id: row.get::<_, Option<String>>(0)?,
            issue_key: row.get(1)?,
            started_at: row.get(2)?,
            stopped_at: row.get(3)?,
            time_spent_seconds: row.get(4)?,
            comment: row.get(5)?,
            tags: row
                .get::<_, Option<String>>(6)?
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default(),
            synced_to_provider: row.get(7)?,
            provider_worklog_id: row.get(8)?,
            created_at: row.get(9)?,
            updated_at: row.get(10)?,
            deleted_at: row.get(11)?,
            last_synced_at: row.get(12)?,
            version: row.get(13)?,
            has_conflict: row.get(14)?,
        })
    }
}

#[async_trait]
impl worklog_core::WorklogRepository for WorklogRepository {
    async fn add(&self, entry: &WorklogEntry) -> WorklogResult<String> {
        let entry = entry.clone();
        let conn = self.connection.clone();

        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().map_err(|_| Self::lock_error())?;

            // Generate or use existing ID
            let id = entry
                .id
                .clone()
                .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

            conn.execute(
                "INSERT INTO worklogs (id, issue_key, started_at, stopped_at, time_spent_seconds,
                                      comment, tags, synced_to_provider, provider_worklog_id,
                                      created_at, updated_at, deleted_at, last_synced_at, version,
                                      has_conflict)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                params![
                    &id,
                    entry.issue_key,
                    entry.started_at,
                    entry.stopped_at,
                    entry.time_spent_seconds,
                    entry.comment,
                    serde_json::to_string(&entry.tags).unwrap_or_else(|_| "[]".to_string()),
                    entry.synced_to_provider,
                    entry.provider_worklog_id,
                    entry.created_at,
                    entry.updated_at,
                    entry.deleted_at,
                    entry.last_synced_at,
                    entry.version,
                    entry.has_conflict,
                ],
            )
            .map_err(Self::to_worklog_error)?;

            // Return the ID that was inserted
            Ok(id)
        })
        .await
        .map_err(|e| worklog_core::WorklogError::StorageError(format!("Task join error: {e}")))?
    }

    async fn update(&self, entry: &WorklogEntry) -> WorklogResult<()> {
        let entry = entry.clone();
        let conn = self.connection.clone();

        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().map_err(|_| Self::lock_error())?;

            conn.execute(
                "UPDATE worklogs SET issue_key = ?, started_at = ?, stopped_at = ?,
                                     time_spent_seconds = ?, comment = ?, tags = ?,
                                     synced_to_provider = ?, provider_worklog_id = ?,
                                     updated_at = ?, deleted_at = ?, last_synced_at = ?, version = ?,
                                     has_conflict = ?
                 WHERE id = ?",
                params![
                    entry.issue_key,
                    entry.started_at,
                    entry.stopped_at,
                    entry.time_spent_seconds,
                    entry.comment,
                    serde_json::to_string(&entry.tags).unwrap_or_else(|_| "[]".to_string()),
                    entry.synced_to_provider,
                    entry.provider_worklog_id,
                    entry.updated_at,
                    entry.deleted_at,
                    entry.last_synced_at,
                    entry.version,
                    entry.has_conflict,
                    entry.id.as_deref().ok_or_else(|| {
                        worklog_core::WorklogError::ValidationError("Entry has no ID".to_string())
                    })?,
                ],
            )
            .map_err(Self::to_worklog_error)?;

            Ok(())
        })
        .await
        .map_err(|e| worklog_core::WorklogError::StorageError(format!("Task join error: {e}")))?
    }

    async fn find_by_id(&self, id: &str) -> WorklogResult<Option<WorklogEntry>> {
        let id = id.to_string();
        let conn = self.connection.clone();

        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().map_err(|_| Self::lock_error())?;

            conn.query_row(
                "SELECT id, issue_key, started_at, stopped_at, time_spent_seconds,
                        comment, tags, synced_to_provider, provider_worklog_id,
                        created_at, updated_at, deleted_at, last_synced_at, version, has_conflict
                 FROM worklogs WHERE id = ? AND deleted_at IS NULL",
                params![id],
                Self::map_row,
            )
            .optional()
            .map_err(Self::to_worklog_error)
        })
        .await
        .map_err(|e| worklog_core::WorklogError::StorageError(format!("Task join error: {e}")))?
    }

    async fn find_active(&self) -> WorklogResult<Option<WorklogEntry>> {
        let conn = self.connection.clone();

        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().map_err(|_| Self::lock_error())?;

            conn.query_row(
                "SELECT id, issue_key, started_at, stopped_at, time_spent_seconds,
                        comment, tags, synced_to_provider, provider_worklog_id,
                        created_at, updated_at, deleted_at, last_synced_at, version, has_conflict
                 FROM worklogs WHERE stopped_at IS NULL AND deleted_at IS NULL LIMIT 1",
                [],
                Self::map_row,
            )
            .optional()
            .map_err(Self::to_worklog_error)
        })
        .await
        .map_err(|e| worklog_core::WorklogError::StorageError(format!("Task join error: {e}")))?
    }

    async fn find_all(&self) -> WorklogResult<Vec<WorklogEntry>> {
        let conn = self.connection.clone();

        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().map_err(|_| Self::lock_error())?;

            let mut stmt = conn
                .prepare(
                    "SELECT id, issue_key, started_at, stopped_at, time_spent_seconds,
                            comment, tags, synced_to_provider, provider_worklog_id,
                            created_at, updated_at, deleted_at, last_synced_at, version, has_conflict
                     FROM worklogs WHERE deleted_at IS NULL ORDER BY started_at DESC",
                )
                .map_err(Self::to_worklog_error)?;

            let entries = stmt
                .query_map([], Self::map_row)
                .map_err(Self::to_worklog_error)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(Self::to_worklog_error)?;

            Ok(entries)
        })
        .await
        .map_err(|e| worklog_core::WorklogError::StorageError(format!("Task join error: {e}")))?
    }

    async fn find_after(&self, after: DateTime<Local>) -> WorklogResult<Vec<WorklogEntry>> {
        let conn = self.connection.clone();

        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().map_err(|_| Self::lock_error())?;

            let mut stmt = conn
                .prepare(
                    "SELECT id, issue_key, started_at, stopped_at, time_spent_seconds,
                            comment, tags, synced_to_provider, provider_worklog_id,
                            created_at, updated_at, deleted_at, last_synced_at, version, has_conflict
                     FROM worklogs WHERE started_at > ? AND deleted_at IS NULL ORDER BY started_at DESC",
                )
                .map_err(Self::to_worklog_error)?;

            let entries = stmt
                .query_map(params![after], Self::map_row)
                .map_err(Self::to_worklog_error)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(Self::to_worklog_error)?;

            Ok(entries)
        })
        .await
        .map_err(|e| worklog_core::WorklogError::StorageError(format!("Task join error: {e}")))?
    }

    async fn find_by_issue(&self, issue_key: &str) -> WorklogResult<Vec<WorklogEntry>> {
        let issue_key = issue_key.to_string();
        let conn = self.connection.clone();

        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().map_err(|_| Self::lock_error())?;

            let mut stmt = conn
                .prepare(
                    "SELECT id, issue_key, started_at, stopped_at, time_spent_seconds,
                            comment, tags, synced_to_provider, provider_worklog_id,
                            created_at, updated_at, deleted_at, last_synced_at, version, has_conflict
                     FROM worklogs WHERE issue_key = ? AND deleted_at IS NULL ORDER BY started_at DESC",
                )
                .map_err(Self::to_worklog_error)?;

            let entries = stmt
                .query_map(params![issue_key], Self::map_row)
                .map_err(Self::to_worklog_error)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(Self::to_worklog_error)?;

            Ok(entries)
        })
        .await
        .map_err(|e| worklog_core::WorklogError::StorageError(format!("Task join error: {e}")))?
    }

    async fn find_unsynced(&self) -> WorklogResult<Vec<WorklogEntry>> {
        let conn = self.connection.clone();

        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().map_err(|_| Self::lock_error())?;

            let mut stmt = conn
                .prepare(
                    "SELECT id, issue_key, started_at, stopped_at, time_spent_seconds,
                            comment, tags, synced_to_provider, provider_worklog_id,
                            created_at, updated_at, deleted_at, last_synced_at, version, has_conflict
                     FROM worklogs
                     WHERE stopped_at IS NOT NULL
                       AND synced_to_provider = 0
                       AND issue_key IS NOT NULL
                       AND deleted_at IS NULL
                     ORDER BY started_at DESC",
                )
                .map_err(Self::to_worklog_error)?;

            let entries = stmt
                .query_map([], Self::map_row)
                .map_err(Self::to_worklog_error)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(Self::to_worklog_error)?;

            Ok(entries)
        })
        .await
        .map_err(|e| worklog_core::WorklogError::StorageError(format!("Task join error: {e}")))?
    }

    /// Delete a worklog entry (soft delete - marks as deleted, doesn't remove)
    ///
    /// This performs a soft delete by:
    /// 1. Setting `deleted_at` to current time
    /// 2. Incrementing the version
    /// 3. Creating a tombstone if the entry was synced to a provider
    ///
    /// Soft deletes prevent resurrection when syncing from remote trackers.
    async fn delete(&self, id: &str) -> WorklogResult<()> {
        let id = id.to_string();
        let conn = self.connection.clone();

        tokio::task::spawn_blocking(move || {
            let mut conn = conn.lock().map_err(|_| Self::lock_error())?;
            let tx = conn
                .transaction()
                .map_err(Self::to_worklog_error)?;

            // Get the entry to check if it's synced
            let entry: Option<(Option<String>, bool)> = tx
                .query_row(
                    "SELECT provider_worklog_id, synced_to_provider FROM worklogs WHERE id = ?",
                    params![id],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .optional()
                .map_err(Self::to_worklog_error)?;

            if let Some((provider_worklog_id, synced_to_provider)) = entry {
                // Soft delete the entry
                let now = Local::now();
                tx.execute(
                    "UPDATE worklogs SET deleted_at = ?, updated_at = ?, version = version + 1 WHERE id = ?",
                    params![now, now, id],
                )
                .map_err(Self::to_worklog_error)?;

                // Create tombstone if it was synced
                if synced_to_provider {
                    if let Some(provider_id) = provider_worklog_id {
                        // Tombstone expires after 30 days
                        let expires_at = now + chrono::Duration::days(30);
                        tx.execute(
                            "INSERT OR REPLACE INTO worklog_tombstones (provider_worklog_id, deleted_at, tracker_name, expires_at)
                             VALUES (?, ?, 'jira', ?)",
                            params![provider_id, now, expires_at],
                        )
                        .map_err(Self::to_worklog_error)?;
                    }
                }
            }

            tx.commit().map_err(Self::to_worklog_error)?;
            Ok(())
        })
        .await
        .map_err(|e| worklog_core::WorklogError::StorageError(format!("Task join error: {e}")))?
    }

    async fn count(&self) -> WorklogResult<i64> {
        let conn = self.connection.clone();

        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().map_err(|_| Self::lock_error())?;

            let count: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM worklogs WHERE deleted_at IS NULL",
                    [],
                    |row| row.get(0),
                )
                .map_err(Self::to_worklog_error)?;

            Ok(count)
        })
        .await
        .map_err(|e| worklog_core::WorklogError::StorageError(format!("Task join error: {e}")))?
    }

    /// Check if a provider worklog ID is tombstoned
    async fn is_tombstoned(&self, provider_worklog_id: &str) -> WorklogResult<bool> {
        let provider_worklog_id = provider_worklog_id.to_string();
        let conn = self.connection.clone();

        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().map_err(|_| Self::lock_error())?;

            let exists: bool = conn
                .query_row(
                    "SELECT COUNT(*) > 0 FROM worklog_tombstones WHERE provider_worklog_id = ?",
                    params![provider_worklog_id],
                    |row| row.get(0),
                )
                .map_err(Self::to_worklog_error)?;

            Ok(exists)
        })
        .await
        .map_err(|e| worklog_core::WorklogError::StorageError(format!("Task join error: {e}")))?
    }

    /// Clean up expired tombstones
    async fn cleanup_tombstones(&self) -> WorklogResult<usize> {
        let conn = self.connection.clone();

        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().map_err(|_| Self::lock_error())?;

            let now = Local::now();
            let deleted = conn
                .execute(
                    "DELETE FROM worklog_tombstones WHERE expires_at < ?",
                    params![now],
                )
                .map_err(Self::to_worklog_error)?;

            Ok(deleted)
        })
        .await
        .map_err(|e| worklog_core::WorklogError::StorageError(format!("Task join error: {e}")))?
    }

    /// Find entry by provider worklog ID (including soft-deleted)
    async fn find_by_provider_id(
        &self,
        provider_worklog_id: &str,
    ) -> WorklogResult<Option<WorklogEntry>> {
        let provider_worklog_id = provider_worklog_id.to_string();
        let conn = self.connection.clone();

        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().map_err(|_| Self::lock_error())?;

            conn.query_row(
                "SELECT id, issue_key, started_at, stopped_at, time_spent_seconds,
                        comment, tags, synced_to_provider, provider_worklog_id,
                        created_at, updated_at, deleted_at, last_synced_at, version, has_conflict
                 FROM worklogs WHERE provider_worklog_id = ?",
                params![provider_worklog_id],
                Self::map_row,
            )
            .optional()
            .map_err(Self::to_worklog_error)
        })
        .await
        .map_err(|e| worklog_core::WorklogError::StorageError(format!("Task join error: {e}")))?
    }
}
