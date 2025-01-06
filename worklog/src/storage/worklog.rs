use chrono::{DateTime, Local};
use log::debug;
use rusqlite::{named_params, params};
use jira::models::core::IssueKey;
use jira::models::user::User;
use jira::models::worklog::Worklog;
use crate::error::WorklogError;
use crate::types::LocalWorklog;
use crate::storage::dbms::Dbms;
impl Dbms {

    ///
    /// # Errors
    /// Returns an error something goes wrong
    pub fn remove_worklog_entry(&self, wl: &Worklog) -> Result<(), WorklogError> {
        self.remove_entry_by_worklog_id(wl.id.as_str())?;
        Ok(())
    }

    ///
    /// # Errors
    /// Returns an error something goes wrong
    pub fn remove_entry_by_worklog_id(&self, wl_id: &str) -> Result<(), WorklogError> {
        self.connection
            .execute("DELETE FROM worklog WHERE id = ?1", params![wl_id])?;
        Ok(())
    }

    /// Adds a new work log entry into the local DBMS
    ///
    /// # Errors
    /// Returns an error something goes wrong
    pub fn add_entry(&self, local_worklog: &LocalWorklog) -> Result<(), WorklogError> {
        debug!("Adding {:?} to DBMS", &local_worklog);

        let result = self.connection.execute(
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
            },).map_err(|e| WorklogError::Sql(format!("Unable to insert into worklog: {e}")))?;

        debug!("With result {}", result);

        Ok(())
    }

    ///
    /// # Errors
    /// Returns an error something goes wrong
    pub fn add_worklog_entries(&self, worklogs: &[LocalWorklog]) -> Result<(), WorklogError> {
        // Prepare the SQL insert statement
        let mut stmt = self.connection.prepare(r"
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

    ///
    /// # Errors
    /// Returns an error something goes wrong
    pub fn get_count(&self) -> Result<i64, WorklogError> {
        let mut stmt = self
            .connection
            .prepare("select count(*) from worklog")
            .map_err(|e| {
                WorklogError::Sql(format!("Unable to retrive count(*) from worklog: {e}"))
            })?;
        let count = stmt.query_row([], |row| row.get(0))?;
        Ok(count)
    }

    ///
    /// # Errors
    /// Returns an error something goes wrong
    pub fn purge_entire_local_worklog(&self) -> Result<(), WorklogError> {
        self.connection.execute("delete from worklog", [])?;
        Ok(())
    }
    ///
    /// # Errors
    /// Returns an error something goes wrong
    ///
    /// # Panics
    /// If the worklog id could not be parsed into an integer
    ///
    pub fn find_worklog_by_id(&self, worklog_id: &str) -> Result<LocalWorklog, WorklogError> {
        let mut stmt = self.connection.prepare("SELECT issue_key, id, author, created, updated, started, time_spent, time_spent_seconds, issue_id, comment FROM worklog WHERE id = ?1")?;
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
    ///
    /// # Errors
    /// Returns an error if something goes wrong
    pub fn find_worklogs_after(
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
        let mut stmt = self.connection.prepare(&sql)?;

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
    use chrono::Days;
    use jira::models::core::Fields;
    use jira::models::issue::IssueSummary;
    use crate::storage::dbms::tests::setup;
    use super::*;
    #[test]
    fn add_worklog_entry() -> Result<(), WorklogError> {
        let worklog = LocalWorklog {
            issue_key: IssueKey::from("ABC-123"),
            id: "1".to_string(),
            author: "Ola Dunk".to_string(),
            created: Local::now(),
            updated: Local::now(),
            started: Local::now(),
            timeSpent: "1h".to_string(),
            timeSpentSeconds: 3600,
            issueId: "1001".to_string(),
            comment: Some("Worked on the issue".to_string()),
        };
        let lws = setup()?;

        lws.add_jira_issues(&vec![IssueSummary {
            id: "123".to_string(),
            key: IssueKey::from("ABC-123"),
            fields: Fields {
                summary: "Test".to_string(),
                ..Default::default()
            },
        }])?;

        lws.add_entry(&worklog)?;

        // Assert
        let result = lws.find_worklog_by_id("1")?;
        assert!(result.id == "1");

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
            issueId: "1001".to_string(),
            comment: Some("Worked on the issue".to_string()),
        };
        let lws = setup()?;
        lws.add_jira_issues(&vec![IssueSummary {
            id: "123".to_string(),
            key: IssueKey::from("ABC-789"),
            fields: Fields {
                summary: "Test".to_string(),
                ..Default::default()
            },
        }])?;

        lws.add_worklog_entries(&[worklog])?;

        // Assert
        let result = lws.find_worklog_by_id("1")?;
        assert!(result.id == "1");

        Ok(())
    }

    #[test]
    fn find_worklogs_after() -> Result<(), WorklogError> {
        let lws = setup()?;

        let worklog = LocalWorklog {
            issue_key: IssueKey::from("ABC-456"),
            id: "1".to_string(),
            author: "John Doe".to_string(),
            created: Local::now(),
            updated: Local::now(),
            started: Local::now(),
            timeSpent: "1h".to_string(),
            timeSpentSeconds: 3600,
            issueId: "1001".to_string(),
            comment: Some("Worked on the issue".to_string()),
        };
        lws.add_jira_issues(&vec![IssueSummary {
            id: "123".to_string(),
            key: IssueKey::from("ABC-456"),
            fields: Fields {
                summary: "Test".to_string(),
                ..Default::default()
            },
        }])?;

        lws.add_entry(&worklog)?;

        let result = lws.find_worklogs_after(
            Local::now().checked_sub_days(Days::new(60)).unwrap(),
            &[],
            &[],
        )?;
        assert!(!result.is_empty(), "No data found in worklog dbms",);
        assert!(!result.is_empty(), "Expected a not empty collection");

        let result = lws.find_worklogs_after(
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