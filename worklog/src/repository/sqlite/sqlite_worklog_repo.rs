use crate::error::WorklogError;
use crate::repository::sqlite::SharedSqliteConnection;
use crate::repository::worklog_repository::WorkLogRepository;
use crate::types::LocalWorklog;
use chrono::{DateTime, Local};
use jira::models::core::IssueKey;
use jira::models::user::User;
use jira::models::worklog::Worklog;
use log::debug;
use rusqlite::{named_params, params, Connection};
use std::sync::Arc;
use std::sync::Mutex;

pub struct SqliteWorklogRepository {
    connection: Arc<Mutex<Connection>>,
}

/// SQL statement to create the `worklog` table.
const CREATE_WORKLOG_TABLE_SQL: &str = r"
    CREATE TABLE IF NOT EXISTS worklog (
        id integer primary key not null,
        issue_key varchar(32),
        issue_id integer,
        author varchar(1024),
        created datetime,
        updated datetime,
        started datetime,
        time_spent varchar(32),
        time_spent_seconds integer,
        comment varchar(1024),
        FOREIGN KEY (issue_id) REFERENCES issue(id) ON DELETE CASCADE
    );
";

/// Creates the `worklog` table in the database.
pub fn create_worklog_table(connection: &SharedSqliteConnection) -> Result<(), WorklogError> {
    let conn = connection.lock().unwrap();
    conn.execute(CREATE_WORKLOG_TABLE_SQL, [])?;
    Ok(())
}

impl SqliteWorklogRepository {
    pub(crate) fn new(connection: Arc<Mutex<Connection>>) -> Self {
        Self { connection }
    }
}

impl WorkLogRepository for SqliteWorklogRepository {
    fn remove_worklog_entry(&self, wl: &Worklog) -> Result<(), WorklogError> {
        self.remove_entry_by_worklog_id(wl.id.as_str())?;
        Ok(())
    }

    fn remove_entry_by_worklog_id(&self, wl_id: &str) -> Result<(), WorklogError> {
        let conn = self
            .connection
            .lock()
            .map_err(|_| WorklogError::LockPoisoned)?;
        conn.execute("DELETE FROM worklog WHERE id = ?1", params![wl_id])?;
        Ok(())
    }

    fn add_entry(&self, local_worklog: &LocalWorklog) -> Result<(), WorklogError> {
        debug!("Adding {:?} to DBMS", &local_worklog);
        let conn = self
            .connection
            .lock()
            .map_err(|_e| WorklogError::LockPoisoned)?;
        let result = conn.execute(
            "INSERT INTO worklog (
            issue_key, id, author, created, updated, started, time_Spent, time_Spent_Seconds, issue_Id, comment
        ) VALUES (
            :issue_key, :id, :author, :created, :updated, :started, :timeSpent, :timeSpentSeconds, :issueId, :comment
        )",
            named_params! {
            ":id" : local_worklog.id,
            ":issue_key": local_worklog.issue_key.to_string(), // No conversion needed
            ":issueId": local_worklog.issueId,
            ":author" : local_worklog.author,
            ":created" : local_worklog.created.to_rfc3339(),
            ":updated" : local_worklog.updated.to_rfc3339(),
            ":started" : local_worklog.started.to_rfc3339(),
            ":timeSpent" : local_worklog.timeSpent,
            ":timeSpentSeconds": local_worklog.timeSpentSeconds,
            ":comment" : local_worklog.comment
            }, ).map_err(|e| WorklogError::Sql(format!("Unable to insert into worklog: {e}")))?;

        debug!("With result {}", result);

        Ok(())
    }

    fn add_worklog_entries(&self, worklogs: &[LocalWorklog]) -> Result<(), WorklogError> {
        let conn = self
            .connection
            .lock()
            .map_err(|_e| WorklogError::LockPoisoned)?;
        // Prepare the SQL insert statement
        let mut stmt = conn.prepare(r"
            INSERT INTO worklog
                (id, issue_key, issue_id, author, created, updated, started, time_spent, time_spent_seconds, comment)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        ")?;

        // Execute the insert statement for each LocalWorklog instance
        for worklog in worklogs {
            stmt.execute(params![
                worklog.id,
                worklog.issue_key.to_string(),
                worklog.issueId,
                worklog.author,
                worklog.created,
                worklog.updated,
                worklog.started,
                worklog.timeSpent,
                worklog.timeSpentSeconds,
                worklog.comment,
            ])?;
        }
        Ok(())
    }

    fn get_count(&self) -> Result<i64, WorklogError> {
        let conn = self
            .connection
            .lock()
            .map_err(|_e| WorklogError::LockPoisoned)?;
        let mut stmt = conn.prepare("select count(*) from worklog").map_err(|e| {
            WorklogError::Sql(format!("Unable to retrive count(*) from worklog: {e}"))
        })?;
        let count = stmt.query_row([], |row| row.get(0))?;
        Ok(count)
    }

    fn purge_entire_local_worklog(&self) -> Result<(), WorklogError> {
        let conn = self
            .connection
            .lock()
            .map_err(|_e| WorklogError::LockPoisoned)?;
        conn.execute("delete from worklog", [])?;
        Ok(())
    }

    fn find_worklog_by_id(&self, worklog_id: &str) -> Result<LocalWorklog, WorklogError> {
        let conn = self
            .connection
            .lock()
            .map_err(|_e| WorklogError::LockPoisoned)?;
        let mut stmt = conn.prepare("SELECT issue_key, id, author, created, updated, started, time_spent, time_spent_seconds, issue_id, comment FROM worklog WHERE id = ?1")?;
        let id: i32 = worklog_id.parse().expect("Invalid number");
        let worklog = stmt.query_row(params![id], |row| {
            Ok(LocalWorklog {
                issue_key: IssueKey::from(row.get::<_, String>(0)?),
                id: row.get::<_, i32>(1)?.to_string(),
                author: row.get(2)?,
                created: row.get(3)?,
                updated: row.get(4)?,
                started: row.get(5)?,
                timeSpent: row.get(6)?,
                timeSpentSeconds: row.get(7)?,
                issueId: row.get(8)?,
                comment: row.get(9)?,
            })
        })?;
        Ok(worklog)
    }

    fn find_worklogs_after(
        &self,
        start_datetime: DateTime<Local>,
        keys_filter: &[IssueKey],
        users_filter: &[User],
    ) -> Result<Vec<LocalWorklog>, WorklogError> {
        // Base SQL query
        let mut sql = String::from(
            "SELECT issue_key, id, author, created, updated, started, time_spent, time_spent_seconds, issue_id, comment
         FROM worklog
         WHERE started > ?1",
        );

        // Dynamic parameters for the query
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(start_datetime.to_rfc3339())];

        // Add `issue_key` filter if `keys` is not empty
        if !keys_filter.is_empty() {
            let placeholders = keys_filter
                .iter()
                .map(|_| "?")
                .collect::<Vec<_>>()
                .join(", ");
            #[allow(clippy::format_push_string)]
            sql.push_str(&format!(" AND issue_key IN ({placeholders})"));

            // Add owned `String` values to the parameters and cast to `Box<dyn ToSql>`
            params.extend(
                keys_filter
                    .iter()
                    .map(|key| Box::new(key.value().to_string()) as Box<dyn rusqlite::ToSql>),
            );
        }
        if !users_filter.is_empty() {
            let placeholders = users_filter
                .iter()
                .map(|_| "?")
                .collect::<Vec<_>>()
                .join(", ");
            #[allow(clippy::format_push_string)]
            sql.push_str(&format!(" AND author IN ({placeholders})"));
            params.extend(
                users_filter
                    .iter()
                    .map(|user| Box::new(user.display_name.clone()) as Box<dyn rusqlite::ToSql>),
            );
        }

        // Convert `params` to a slice of `&dyn ToSql`
        let params_slice: Vec<&dyn rusqlite::ToSql> = params.iter().map(AsRef::as_ref).collect();

        debug!("find_worklogs_after():- {sql}");

        // Prepare the query
        let conn = self
            .connection
            .lock()
            .map_err(|_e| WorklogError::LockPoisoned)?;
        let mut stmt = conn.prepare(&sql)?;

        // Execute the query and map results
        let worklogs = stmt
            .query_map(params_slice.as_slice(), |row| {
                Ok(LocalWorklog {
                    issue_key: IssueKey::new(&row.get::<_, String>(0)?),
                    id: row.get::<_, i32>(1)?.to_string(),
                    author: row.get(2)?,
                    created: row.get(3)?,
                    updated: row.get(4)?,
                    started: row.get(5)?,
                    timeSpent: row.get(6)?,
                    timeSpentSeconds: row.get(7)?,
                    issueId: row.get(8)?,
                    comment: row.get(9)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(worklogs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::issue_repository::IssueRepository;
    use chrono::Days;
    use jira::models::core::Fields;
    use jira::models::issue::IssueSummary;

    use crate::repository::sqlite::tests::test_database_manager;

    const ISSUE_ID: &str = "123";
    #[test]
    fn add_worklog_entry() -> Result<(), WorklogError> {
        let worklog = LocalWorklog {
            id: "123".to_string(),
            issue_key: IssueKey::from("ABC-123"),
            author: "Ola Dunk".to_string(),
            created: Local::now(),
            updated: Local::now(),
            started: Local::now(),
            timeSpent: "1h".to_string(),
            timeSpentSeconds: 3600,
            issueId: ISSUE_ID.parse().unwrap(),
            comment: Some("Worked on the issue".to_string()),
        };

        let db_manager = test_database_manager()?;
        let issue_repo_for_test = db_manager.create_issue_repository();

        issue_repo_for_test.add_jira_issues(&vec![IssueSummary {
            id: 123.to_string(),
            key: IssueKey::from("ABC-123"),
            fields: Fields {
                summary: "Test".to_string(),
                ..Default::default()
            },
        }])?;

        let worklog_repo_for_test = db_manager.create_worklog_repository();

        worklog_repo_for_test.add_entry(&worklog)?;

        // Assert
        let result = worklog_repo_for_test.find_worklog_by_id(ISSUE_ID)?;
        assert_eq!(result.id, ISSUE_ID, "expected id 123, got {}", result.id);

        Ok(())
    }

    #[test]
    fn add_worklog_entries() -> Result<(), WorklogError> {
        let worklog = LocalWorklog {
            issue_key: IssueKey::from("ABC-789"),
            id: "1".to_string(),
            author: "John Doe".to_string(),
            created: Local::now(),
            updated: Local::now(),
            started: Local::now(),
            timeSpent: "1h".to_string(),
            timeSpentSeconds: 3600,
            issueId: ISSUE_ID.parse().unwrap(),
            comment: Some("Worked on the issue".to_string()),
        };
        let db_manager = test_database_manager()?;
        let issue_repo = db_manager.create_issue_repository();
        issue_repo.add_jira_issues(&vec![IssueSummary {
            id: ISSUE_ID.to_string(),
            key: IssueKey::from("ABC-789"),
            fields: Fields {
                summary: "Test".to_string(),
                ..Default::default()
            },
        }])?;

        let worklog_repo = db_manager.create_worklog_repository();
        worklog_repo.add_worklog_entries(&[worklog])?;

        // Assert
        let result = worklog_repo.find_worklog_by_id("1")?;
        assert_eq!(result.id, "1");

        Ok(())
    }

    #[test]
    fn find_worklogs_after() -> Result<(), WorklogError> {
        let db_manager = test_database_manager()?;

        let worklog = LocalWorklog {
            issue_key: IssueKey::from("ABC-456"),
            id: "1".to_string(),
            author: "John Doe".to_string(),
            created: Local::now(),
            updated: Local::now(),
            started: Local::now(),
            timeSpent: "1h".to_string(),
            timeSpentSeconds: 3600,
            issueId: ISSUE_ID.parse().unwrap(),
            comment: Some("Worked on the issue".to_string()),
        };
        let test_issue_repo = db_manager.create_issue_repository();
        test_issue_repo.add_jira_issues(&vec![IssueSummary {
            id: 123.to_string(),
            key: IssueKey::from("ABC-456"),
            fields: Fields {
                summary: "Test".to_string(),
                ..Default::default()
            },
        }])?;

        let test_worklog_repo = db_manager.create_worklog_repository();
        test_worklog_repo.add_entry(&worklog)?;

        let result = test_worklog_repo.find_worklogs_after(
            Local::now().checked_sub_days(Days::new(60)).unwrap(),
            &[],
            &[],
        )?;
        assert!(!result.is_empty(), "No data found in worklog dbms",);
        assert!(!result.is_empty(), "Expected a not empty collection");

        let result = test_worklog_repo.find_worklogs_after(
            Local::now().checked_sub_days(Days::new(60)).unwrap(),
            &[],
            &[User {
                display_name: "John Doe".to_string(),
                ..Default::default()
            }],
        )?;
        assert!(
            !result.is_empty(),
            "No data found in worklog dbms for John Doe",
        );
        Ok(())
    }
}
