use async_trait::async_trait;
use rusqlite::{params, Connection, OptionalExtension};
use std::sync::{Arc, Mutex};
use worklog_core::{User, WorklogResult};

pub(crate) struct UserRepository {
    connection: Arc<Mutex<Connection>>,
}

#[allow(clippy::needless_pass_by_value)]
impl UserRepository {
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
impl worklog_core::UserRepository for UserRepository {
    async fn save_current_user(&self, user: &User) -> WorklogResult<()> {
        let user = user.clone();
        let conn = self.connection.clone();

        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().map_err(|_| Self::lock_error())?;

            let now = chrono::Local::now().to_rfc3339();

            conn.execute(
                "INSERT OR REPLACE INTO users (id, display_name, email, timezone, created_at, updated_at)
                 VALUES (?, ?, ?, ?, ?, ?)",
                params![
                    &user.id,
                    &user.display_name,
                    &user.email,
                    &user.timezone,
                    &now,
                    &now,
                ],
            )
            .map_err(Self::to_worklog_error)?;

            Ok(())
        })
        .await
        .map_err(|e| worklog_core::WorklogError::StorageError(format!("Task join error: {e}")))?
    }

    async fn get_current_user(&self) -> WorklogResult<Option<User>> {
        let conn = self.connection.clone();

        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().map_err(|_| Self::lock_error())?;

            conn.query_row(
                "SELECT id, display_name, email, timezone FROM users LIMIT 1",
                [],
                |row| {
                    Ok(User {
                        id: row.get(0)?,
                        display_name: row.get(1)?,
                        email: row.get(2)?,
                        timezone: row.get(3)?,
                    })
                },
            )
            .optional()
            .map_err(Self::to_worklog_error)
        })
        .await
        .map_err(|e| worklog_core::WorklogError::StorageError(format!("Task join error: {e}")))?
    }
}
