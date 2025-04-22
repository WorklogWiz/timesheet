use crate::error::WorklogError;
use crate::repository::sqlite::SharedSqliteConnection;
use crate::repository::timer_repository::TimerRepository;
use crate::types::Timer;
use chrono::{DateTime, Local, Utc};
use log::debug;
use rusqlite::{params, Connection, Result as SqliteResult};
use std::sync::Arc;
use std::sync::Mutex;

pub struct SqliteTimerRepository {
    connection: Arc<Mutex<Connection>>,
}

/// SQL statement to create the `worklog` table.
const CREATE_TIMER_TABLE_SQL: &str = r"
    CREATE TABLE IF NOT EXISTS timer (
        id integer primary key not null,
        issue_key varchar(32),
        created datetime,
        started datetime,
        end datetime,
        synced boolean,
        comment varchar(1024),
        FOREIGN KEY (issue_key) REFERENCES issue(key) ON DELETE CASCADE
    );
    
    -- Partial index to find the active timer and enforce a single active timer at a time
    CREATE UNIQUE INDEX IF NOT EXISTS idx_single_active_timer ON timer ((end IS NULL)) WHERE end IS NULL;
";

/// Creates the `timer` table in the database.
pub fn create_timer_table(connection: &SharedSqliteConnection) -> Result<(), WorklogError> {
    let conn = connection.lock().unwrap();
    conn.execute_batch(CREATE_TIMER_TABLE_SQL)?;
    Ok(())
}

impl SqliteTimerRepository {
    pub(crate) fn new(connection: Arc<Mutex<Connection>>) -> Self {
        Self { connection }
    }
}

impl TimerRepository for SqliteTimerRepository {
    fn start_timer(&self, timer: &Timer) -> Result<i64, WorklogError> {
        debug!("Starting timer for issue {}", timer.issue_key);
        let conn = self
            .connection
            .lock()
            .map_err(|_| WorklogError::LockPoisoned)?;
        let result: SqliteResult<i64> = conn.query_row(
            r"INSERT INTO timer (issue_key, created, started, end, synced, comment)
              VALUES (?, ?, ?, ?, ?, ?)
              RETURNING id",
            params![
                timer.issue_key,
                timer.created_at,
                timer.started_at,
                timer.stopped_at,
                timer.synced,
                timer.comment,
            ],
            |row| row.get(0),
        );

        match result {
            Ok(id) => Ok(id),
            Err(err) => {
                // Check if error is due to the unique constraint (active timer already exists)
                if let rusqlite::Error::SqliteFailure(error, Some(message)) = &err {
                    if error.extended_code == 2067 && message.contains("idx_single_active_timer") {
                        return Err(WorklogError::ActiveTimerExists);
                    }
                }
                Err(WorklogError::DatabaseError(err.to_string()))
            }
        }
    }

    fn find_active_timer(&self) -> Result<Option<Timer>, WorklogError> {
        let conn = self
            .connection
            .lock()
            .map_err(|_| WorklogError::DatabaseLockError)?;

        let result = conn.query_row(
            r"SELECT id, issue_key, created, started, end, synced, comment 
              FROM timer 
              WHERE end IS NULL",
            [],
            |row| {
                Ok(Timer {
                    id: Some(row.get(0)?),
                    issue_key: row.get(1)?,
                    created_at: row.get(2)?,
                    started_at: row.get(3)?,
                    stopped_at: row.get(4)?,
                    synced: row.get(5)?,
                    comment: row.get(6)?,
                })
            },
        );

        match result {
            Ok(timer) => Ok(Some(timer)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(err) => Err(WorklogError::DatabaseError(err.to_string())),
        }
    }

    /// Stops the currently active timer
    fn stop_active_timer(&self, stop_time: Option<DateTime<Local>>) -> Result<Timer, WorklogError> {
        // Find the active timer
        let mut active_timer = match self.find_active_timer()? {
            Some(timer) => timer,
            None => return Err(WorklogError::NoActiveTimer),
        };

        let conn = self
            .connection
            .lock()
            .map_err(|_| WorklogError::DatabaseLockError)?;

        // Set the stop time to supplied value or now
        active_timer.stopped_at = stop_time.or_else(|| Some(Utc::now().with_timezone(&Local)));
        
        debug!("Stopping timer {:?}", &active_timer);
        // Update the timer in the database
        conn.execute(
            "UPDATE timer SET end = ? WHERE id = ?",
            params![active_timer.stopped_at, active_timer.id],
        )?;

        debug!("Stopped timer for issue {}", active_timer.issue_key);

        Ok(active_timer)
    }
    /// Finds all timers for a specific issue
    fn find_by_issue_key(&self, issue_ke: &str) -> Result<Vec<Timer>, WorklogError> {
        let conn = self
            .connection
            .lock()
            .map_err(|_| WorklogError::DatabaseLockError)?;

        let mut stmt = conn.prepare(
            r"SELECT id, issue_key, created, started, end, synced, comment 
              FROM timer 
              WHERE issue_ke = ? 
              ORDER BY started DESC",
        )?;

        let timer_iter = stmt.query_map(params![issue_ke], |row| {
            Ok(Timer {
                id: Some(row.get(0)?),
                issue_key: row.get(1)?,
                created_at: row.get(2)?,
                started_at: row.get(3)?,
                stopped_at: row.get(4)?,
                synced: row.get(5)?,
                comment: row.get(6)?,
            })
        })?;

        let mut timers = Vec::new();
        for timer_result in timer_iter {
            timers.push(timer_result?);
        }

        Ok(timers)
    }

    /// Finds all timers that started after a specific date
    fn find_after_date(&self, date: DateTime<Utc>) -> Result<Vec<Timer>, WorklogError> {
        let conn = self
            .connection
            .lock()
            .map_err(|_| WorklogError::DatabaseLockError)?;

        let mut stmt = conn.prepare(
            r"SELECT id, issue_key, created, started, end, synced, comment 
              FROM timer 
              WHERE started >= ? 
              ORDER BY started DESC",
        )?;

        let timer_iter = stmt.query_map(params![date], |row| {
            Ok(Timer {
                id: Some(row.get(0)?),
                issue_key: row.get(1)?,
                created_at: row.get(2)?,
                started_at: row.get(3)?,
                stopped_at: row.get(4)?,
                synced: row.get(5)?,
                comment: row.get(6)?,
            })
        })?;

        let mut timers = Vec::new();
        for timer_result in timer_iter {
            timers.push(timer_result?);
        }

        Ok(timers)
    }

    /// Deletes a timer by its ID
    fn delete(&self, id: i64) -> Result<(), WorklogError> {
        let conn = self
            .connection
            .lock()
            .map_err(|_| WorklogError::DatabaseLockError)?;

        let rows_affected = conn.execute("DELETE FROM timer WHERE id = ?", params![id])?;

        if rows_affected == 0 {
            return Err(WorklogError::TimerNotFound(id));
        }

        Ok(())
    }

    /// Updates an existing timer in the database
    fn update(&self, timer: &Timer) -> Result<(), WorklogError> {
        if timer.id.is_none() {
            return Err(WorklogError::InvalidTimerData(
                "Cannot update timer without ID".to_string(),
            ));
        }

        let conn = self
            .connection
            .lock()
            .map_err(|_| WorklogError::DatabaseLockError)?;

        let rows_affected = conn.execute(
            r"UPDATE timer 
              SET issue_key = ?, created = ?, started = ?, end = ?, synced = ?, comment = ? 
              WHERE id = ?",
            params![
                timer.issue_key,
                timer.created_at,
                timer.started_at,
                timer.stopped_at,
                timer.synced,
                timer.comment,
                timer.id,
            ],
        )?;

        if rows_affected == 0 {
            return Err(WorklogError::TimerNotFound(timer.id.unwrap()));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::issue_repository::IssueRepository;
    use crate::repository::sqlite::tests::test_database_manager;
    use jira::models::core::{Fields, IssueKey};
    use jira::models::issue::IssueSummary;

    const ISSUE_KEY: &str = "ABC-123";
    #[test]
    fn start_timer_test() -> Result<(), WorklogError> {
        let db_manager = test_database_manager()?;
        let issue_repo_for_test = db_manager.create_issue_repository();

        issue_repo_for_test.add_jira_issues(&vec![IssueSummary {
            id: "123".to_string(),
            key: IssueKey::from("ABC-123"),
            fields: Fields {
                summary: "Test".to_string(),
                ..Default::default()
            },
        }])?;

        let worklog_repo_for_test = db_manager.create_timer_repository();

        let timer = Timer::start_new(ISSUE_KEY.to_string());
        let result = worklog_repo_for_test.start_timer(&timer)?;

        // Assert
        assert!(result > 0, "Timer id should be greater than 0");

        Ok(())
    }
    // TODO: Add more tests for sqlite_timer_repo
}
