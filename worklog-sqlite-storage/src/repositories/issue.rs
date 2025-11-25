use async_trait::async_trait;
use rusqlite::{params, Connection, OptionalExtension};
use std::sync::{Arc, Mutex};
use worklog_core::{Issue, WorklogResult};

pub(crate) struct IssueRepository {
    connection: Arc<Mutex<Connection>>,
}

#[allow(clippy::needless_pass_by_value)]
impl IssueRepository {
    pub(crate) fn new(connection: Arc<Mutex<Connection>>) -> Self {
        Self { connection }
    }

    fn to_worklog_error(e: rusqlite::Error) -> worklog_core::WorklogError {
        worklog_core::WorklogError::StorageError(e.to_string())
    }

    fn lock_error() -> worklog_core::WorklogError {
        worklog_core::WorklogError::StorageError("Database lock poisoned".to_string())
    }
}

#[async_trait]
impl worklog_core::IssueRepository for IssueRepository {
    async fn add_issues(&self, issues: &[Issue]) -> WorklogResult<()> {
        let issues = issues.to_vec();
        let conn = self.connection.clone();

        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().map_err(|_| Self::lock_error())?;

            let now = chrono::Local::now().to_rfc3339();

            for issue in &issues {
                conn.execute(
                    "INSERT OR REPLACE INTO issues (issue_key, summary, description, tags, created_at, updated_at)
                     VALUES (?, ?, ?, ?, ?, ?)",
                    params![
                        &issue.key,
                        &issue.summary,
                        &issue.description,
                        serde_json::to_string(&issue.tags).unwrap_or_else(|_| "[]".to_string()),
                        &now,
                        &now,
                    ],
                )
                .map_err(Self::to_worklog_error)?;
            }

            Ok(())
        })
        .await
        .map_err(|e| worklog_core::WorklogError::StorageError(format!("Task join error: {e}")))?
    }

    async fn find_by_key(&self, key: &str) -> WorklogResult<Option<Issue>> {
        let key = key.to_string();
        let conn = self.connection.clone();

        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().map_err(|_| Self::lock_error())?;

            conn.query_row(
                "SELECT issue_key, summary, description, tags FROM issues WHERE issue_key = ?",
                params![key],
                |row| {
                    Ok(Issue {
                        key: row.get(0)?,
                        summary: row.get(1)?,
                        description: row.get(2)?,
                        tags: row
                            .get::<_, Option<String>>(3)?
                            .and_then(|s| serde_json::from_str(&s).ok())
                            .unwrap_or_default(),
                        provider_id: None, // Not stored in our DB
                    })
                },
            )
            .optional()
            .map_err(Self::to_worklog_error)
        })
        .await
        .map_err(|e| worklog_core::WorklogError::StorageError(format!("Task join error: {e}")))?
    }

    async fn find_by_keys(&self, keys: &[String]) -> WorklogResult<Vec<Issue>> {
        let keys = keys.to_vec();
        let conn = self.connection.clone();

        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().map_err(|_| Self::lock_error())?;

            if keys.is_empty() {
                return Ok(vec![]);
            }

            // Build IN clause
            let placeholders = keys.iter().map(|_| "?").collect::<Vec<_>>().join(",");
            let query = format!(
                "SELECT issue_key, summary, description, tags FROM issues WHERE issue_key IN ({placeholders})"
            );

            let mut stmt = conn.prepare(&query).map_err(Self::to_worklog_error)?;

            let params: Vec<&dyn rusqlite::ToSql> = keys.iter().map(|k| k as &dyn rusqlite::ToSql).collect();

            let issues = stmt
                .query_map(&params[..], |row| {
                    Ok(Issue {
                        key: row.get(0)?,
                        summary: row.get(1)?,
                        description: row.get(2)?,
                        tags: row
                            .get::<_, Option<String>>(3)?
                            .and_then(|s| serde_json::from_str(&s).ok())
                            .unwrap_or_default(),
                        provider_id: None,
                    })
                })
                .map_err(Self::to_worklog_error)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(Self::to_worklog_error)?;

            Ok(issues)
        })
        .await
        .map_err(|e| worklog_core::WorklogError::StorageError(format!("Task join error: {e}")))?
    }

    async fn find_all(&self) -> WorklogResult<Vec<Issue>> {
        let conn = self.connection.clone();

        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().map_err(|_| Self::lock_error())?;

            let mut stmt = conn
                .prepare(
                    "SELECT issue_key, summary, description, tags FROM issues ORDER BY issue_key",
                )
                .map_err(Self::to_worklog_error)?;

            let issues = stmt
                .query_map([], |row| {
                    Ok(Issue {
                        key: row.get(0)?,
                        summary: row.get(1)?,
                        description: row.get(2)?,
                        tags: row
                            .get::<_, Option<String>>(3)?
                            .and_then(|s| serde_json::from_str(&s).ok())
                            .unwrap_or_default(),
                        provider_id: None,
                    })
                })
                .map_err(Self::to_worklog_error)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(Self::to_worklog_error)?;

            Ok(issues)
        })
        .await
        .map_err(|e| worklog_core::WorklogError::StorageError(format!("Task join error: {e}")))?
    }

    async fn search(&self, query: &str) -> WorklogResult<Vec<Issue>> {
        let query = query.to_string();
        let conn = self.connection.clone();

        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().map_err(|_| Self::lock_error())?;

            let search_pattern = format!("%{query}%");

            let mut stmt = conn
                .prepare(
                    "SELECT issue_key, summary, description, tags FROM issues
                     WHERE issue_key LIKE ? OR summary LIKE ? OR description LIKE ? OR tags LIKE ?
                     ORDER BY issue_key",
                )
                .map_err(Self::to_worklog_error)?;

            let issues = stmt
                .query_map(
                    params![
                        &search_pattern,
                        &search_pattern,
                        &search_pattern,
                        &search_pattern
                    ],
                    |row| {
                        Ok(Issue {
                            key: row.get(0)?,
                            summary: row.get(1)?,
                            description: row.get(2)?,
                            tags: row
                                .get::<_, Option<String>>(3)?
                                .and_then(|s| serde_json::from_str(&s).ok())
                                .unwrap_or_default(),
                            provider_id: None,
                        })
                    },
                )
                .map_err(Self::to_worklog_error)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(Self::to_worklog_error)?;

            Ok(issues)
        })
        .await
        .map_err(|e| worklog_core::WorklogError::StorageError(format!("Task join error: {e}")))?
    }

    async fn find_keys_with_worklogs(&self) -> WorklogResult<Vec<String>> {
        let conn = self.connection.clone();

        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().map_err(|_| Self::lock_error())?;

            let mut stmt = conn
                .prepare(
                    "SELECT DISTINCT issue_key FROM worklogs
                     WHERE issue_key IS NOT NULL
                     ORDER BY issue_key",
                )
                .map_err(Self::to_worklog_error)?;

            let keys = stmt
                .query_map([], |row| row.get(0))
                .map_err(Self::to_worklog_error)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(Self::to_worklog_error)?;

            Ok(keys)
        })
        .await
        .map_err(|e| worklog_core::WorklogError::StorageError(format!("Task join error: {e}")))?
    }

    async fn delete(&self, key: &str) -> WorklogResult<()> {
        let key = key.to_string();
        let conn = self.connection.clone();

        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().map_err(|_| Self::lock_error())?;

            conn.execute("DELETE FROM issues WHERE issue_key = ?", params![key])
                .map_err(Self::to_worklog_error)?;

            Ok(())
        })
        .await
        .map_err(|e| worklog_core::WorklogError::StorageError(format!("Task join error: {e}")))?
    }
}
