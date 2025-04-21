use crate::error::WorklogError;
use crate::repository::sqlite::SharedSqliteConnection;
use crate::repository::timer_repository::TimerRepository;
use crate::repository::worklog_repository::WorkLogRepository;
use crate::types::{LocalWorklog, Timer};
use chrono::{DateTime, Local};
use jira::models::core::IssueKey;
use jira::models::user::User;
use jira::models::worklog::Worklog;
use log::debug;
use rusqlite::{named_params, params, Connection, Result as SqliteResult};
use std::sync::Arc;
use std::sync::Mutex;

pub struct SqliteTimerRepository {
    connection: Arc<Mutex<Connection>>,
}

/// SQL statement to create the `worklog` table.
const CREATE_TIMER_TABLE_SQL: &str = r"
    CREATE TABLE IF NOT EXISTS timer (
        id integer primary key not null,
        issue_id integer,
        created datetime,
        started datetime,
        end datetime,
        synced boolean,
        comment varchar(1024),
        FOREIGN KEY (issue_id) REFERENCES issue(id) ON DELETE CASCADE
    );
    
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
    fn remove_timer_entry(&self, id: i64) -> Result<(), WorklogError> {
        todo!()
    }

    fn start_timer(&self, timer: &Timer) -> Result<(i64), WorklogError> {
        debug!("Starting timer for issue {}", timer.issue_id);
        let conn = self.connection.lock().map_err(|_| WorklogError::LockPoisoned)?;
        let result: SqliteResult<i64> = conn.query_row(
            r"INSERT INTO timer (issue_id, created, started, end, synced, comment)
              VALUES (?, ?, ?, ?, ?, ?)
              RETURNING id",
            params![
                timer.issue_id,
                timer.created_at,
                timer.started_at,
                timer.stopped_at,
                timer.synced,
                timer.comment,
            ],
            |row| row.get(0)
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

    fn find_current_timer(&self) -> Result<Option<Timer>, WorklogError> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::issue_repository::IssueRepository;
    use jira::models::core::Fields;
    use jira::models::issue::IssueSummary;

    use crate::repository::sqlite::tests::test_database_manager;

    const ISSUE_ID: &str = "123";
    #[test]
    fn start_timer_test() -> Result<(), WorklogError> {

        let db_manager = test_database_manager()?;
        let issue_repo_for_test = db_manager.create_issue_repository();

        issue_repo_for_test.add_jira_issues(&vec![IssueSummary {
            id: ISSUE_ID.to_string(),
            key: IssueKey::from("ABC-123"),
            fields: Fields {
                summary: "Test".to_string(),
                ..Default::default()
            },
        }])?;

        let worklog_repo_for_test = db_manager.create_timer_repository();

        let timer = Timer::start_new(ISSUE_ID.to_string());
        let result = worklog_repo_for_test.start_timer(&timer)?;

        // Assert
        assert!(result > 0, "Timer id should be greater than 0");
        
        Ok(())
    }
}
