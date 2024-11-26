use chrono::{DateTime, Local};
use common::WorklogError;
use jira_lib::{JiraIssue, JiraKey, Worklog};
use log::debug;
use rusqlite::{named_params, params, Connection};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, PartialOrd, Ord, Clone)]
#[allow(non_snake_case)]
pub struct LocalWorklog {
    pub issue_key: JiraKey,
    pub id: String, // Numeric, really
    pub author: String,
    pub created: DateTime<Local>,
    pub updated: DateTime<Local>,
    pub started: DateTime<Local>,
    pub timeSpent: String, // consider migrating to value type
    pub timeSpentSeconds: i32,
    pub issueId: String, // Numeric FK to issue
    pub comment: Option<String>,
}

#[derive(Debug, Serialize,Deserialize,Eq, PartialEq, Clone)]
pub struct JiraIssueInfo {
    pub issue_key: JiraKey,
    pub summary: String,
}

impl LocalWorklog {
    /// Converts a Jira `Worklog` entry into a `LocalWorklog` entry
    #[must_use]
    pub fn from_worklog(worklog: &Worklog, issue_key: JiraKey) -> Self {
        LocalWorklog {
            issue_key,
            id: worklog.id.clone(),
            author: worklog.author.displayName.clone(),
            created: worklog.created.with_timezone(&Local),
            updated: worklog.updated.with_timezone(&Local),
            started: worklog.started.with_timezone(&Local),
            timeSpent: worklog.timeSpent.clone(),
            timeSpentSeconds: worklog.timeSpentSeconds,
            issueId: worklog.issueId.clone(),
            comment: worklog.comment.clone(),
        }
    }
}

pub struct LocalWorklogService {
    dbms_path: PathBuf,
    connection: Connection,
}


impl LocalWorklogService {

    pub fn add_jira_issues(&self, jira_issues: &Vec<JiraIssue>) -> Result<(),WorklogError> {
        let mut stmt = self.connection.prepare(
            "INSERT INTO jira_issue (issue_key, summary)
         VALUES (?1, ?2)
         ON CONFLICT(issue_key) DO UPDATE SET summary = excluded.summary",
        )?;
        for issue in jira_issues {
            if let Err(e) = stmt.execute(params![issue.key.to_string(), issue.fields.summary]) {
                panic!("Unable to insert jira_issue({},{}): {}", issue.key, issue.fields.summary,e);
            }
        }
        Ok(())
    }

    pub fn get_jira_issues_filtered_by_keys(&self,
                                            keys: Vec<JiraKey>,
    ) -> Result<Vec<JiraIssueInfo>, WorklogError> {
        if keys.is_empty() {
            // Return an empty vector if no keys are provided
            return Ok(Vec::new());
        }

        debug!("selecting jira_issue from database for keys {:?}", keys);

        // Build the `IN` clause dynamically
        let placeholders = keys.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
        let sql = format!(
            "SELECT issue_key, summary
         FROM jira_issue
         WHERE issue_key IN ({})",
            placeholders
        );

        // Prepare the parameters for the query
        let params: Vec<String> = keys.iter().map(|key| key.to_string()).collect();

        let mut stmt = self.connection.prepare(&sql)?;

        let issues = stmt
            .query_map(rusqlite::params_from_iter(params), |row| {
                Ok(JiraIssueInfo {
                    issue_key: JiraKey::new(&row.get::<_, String>(0)?),
                    summary: row.get(1)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(issues)
    }

    ///
    /// # Errors
    /// Returns an error something goes wrong
    pub fn remove_entry(&self, wl: &Worklog) -> Result<(), WorklogError> {
        self.remove_entry_by_worklog_id(wl.id.as_str())?;
        Ok(())
    }
    ///
    /// # Errors
    /// Returns an error something goes wrong
    pub fn remove_entry_by_worklog_id(&self, wl_id: &str) -> Result<(), WorklogError> {
        self.connection
            .execute("delete from worklog where id = ?1", params![wl_id])?;
        Ok(())
    }
}

impl LocalWorklogService {
    ///
    /// # Errors
    /// Returns an error something goes wrong
    pub fn new(dbms_path: &Path) -> Result<Self, WorklogError> {
        let connection = Connection::open(dbms_path).map_err(|e| WorklogError::OpenDbms {
            path: dbms_path.to_string_lossy().into(),
            reason: e.to_string(),
        })?;
        // Creates the schema if needed
        create_local_worklog_schema(&connection)?;

        Ok(LocalWorklogService {
            connection,
            dbms_path: PathBuf::from(dbms_path),
        })
    }

    /// Adds a new entry into the local DBMS
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
    pub fn add_worklog_entries(&self, worklogs: Vec<LocalWorklog>) -> Result<(), WorklogError> {
        // Prepare the SQL insert statement
        let mut stmt = self.connection.prepare("INSERT INTO worklog (id, issue_key, issue_id, author, created, updated, started, time_spent, time_spent_seconds, comment) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)")?;

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

    /// Retrieves the path to the current open database
    pub fn get_dbms_path(&self) -> &PathBuf {
        &self.dbms_path
    }

    ///
    /// # Errors
    /// Returns an error something goes wrong
    // TODO: Replace Jira keys as String with the type JiraKey
    pub fn find_unique_keys(&self) -> Result<Vec<String>, WorklogError> {
        let mut stmt = self
            .connection
            .prepare("SELECT DISTINCT(issue_key) FROM worklog order by issue_key asc")?;
        let issue_keys: Vec<String> = stmt
            .query_map([], |row| row.get::<_, String>(0))?
            .filter_map(Result::ok)
            .collect();
        Ok(issue_keys)
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
                issue_key: JiraKey::from(row.get::<_, String>(0)?),
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
        keys: &Vec<JiraKey>,
    ) -> Result<Vec<LocalWorklog>, rusqlite::Error> {
        // Base SQL query
        let mut sql = String::from(
            "SELECT issue_key, id, author, created, updated, started, time_spent, time_spent_seconds, issue_id, comment
         FROM worklog
         WHERE started > ?1",
        );

        // Dynamic parameters for the query
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(start_datetime.to_rfc3339())];

        // Add `issue_key` filter if `keys` is not empty
        if !keys.is_empty() {
            let placeholders = keys.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
            sql.push_str(&format!(" AND issue_key IN ({})", placeholders));

            // Add owned `String` values to the parameters and cast to `Box<dyn ToSql>`
            params.extend(
                keys.into_iter()
                    .map(|key| Box::new(key.value().to_string()) as Box<dyn rusqlite::ToSql>),
            );
        }

        // Convert `params` to a slice of `&dyn ToSql`
        let params_slice: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();

        // Prepare the query
        let mut stmt = self.connection.prepare(&sql)?;

        // Execute the query and map results
        let worklogs = stmt
            .query_map(params_slice.as_slice(), |row| {
                Ok(LocalWorklog {
                    issue_key: JiraKey::new(&row.get::<_, String>(0)?),
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

///
/// # Errors
/// Returns an error something goes wrong
pub fn create_local_worklog_schema(connection: &Connection) -> Result<(), WorklogError> {
    let sql = r"
        create table if not exists worklog (
            id integer primary key not null,
            issue_key varchar(32),
            issue_id varchar(32),
            author varchar(1024),
            created datetime,
            updated datetime,
            started datetime,
            time_spent varchar(32),
            time_spent_seconds intger,
            comment varchar(1024)
    );
    ";
    connection
        .execute(sql, [])
        .map_err(|e| WorklogError::Sql(format!("Unable to create table 'worklog': {e}")))?;

    let sql = r"
    create table if not exists jira_issue (
        issue_key varchar(32) primary key,
        summary varchar(1024)
    );
    ";
    connection.execute(sql, []).map_err(|e| WorklogError::Sql(format!("Unable to create table 'jira_issue': {e}")))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Days, Local};
    use common::config;
    use jira_lib::JiraFields;

    pub fn setup() -> Result<LocalWorklogService, WorklogError> {
        let tmp_db = config::tmp_local_worklog_dbms_file_name()
            .map_err(|_e| WorklogError::CreateFile("temp file".to_string()))?;

        let local_worklog_service = LocalWorklogService::new(&tmp_db)?;
        Ok(local_worklog_service)
    }

    #[test]
    fn test_open_dbms() -> Result<(), WorklogError> {
        let _local_worklog_service = setup()?;
        Ok(())
    }

    #[ignore]
    fn test_add_local_worklog_entry() -> Result<(), WorklogError> {
        let worklog = LocalWorklog {
            issue_key: JiraKey::from("ABC-123"),
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
        let worklog_service = setup()?;
        worklog_service.add_entry(&worklog)?;

        Ok(())
    }

    #[test]
    fn test_add_local_worklog_entries() -> Result<(), WorklogError> {
        let worklog = LocalWorklog {
            issue_key: JiraKey::from("ABC-123"),
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
        let worklog_service = setup()?;
        worklog_service.add_worklog_entries(vec![worklog])?;
        let _result = worklog_service.find_worklog_by_id("1")?;
        Ok(())
    }

    #[test]
    fn test_find_worklogs_after() -> Result<(), WorklogError> {
        let rt = LocalWorklogService::new(&config::local_worklog_dbms_file_name())?;
        let result =
            rt.find_worklogs_after(Local::now().checked_sub_days(Days::new(60)).unwrap(), &vec![])?;
        assert!(
            !result.is_empty(),
            "No data found in local worklog dbms {}",
            config::local_worklog_dbms_file_name().to_string_lossy()
        );
        assert!(result.len() >= 30);
        Ok(())
    }

    #[test]
    fn test_add_jira_issues() -> Result<(), WorklogError> {
        let lws = setup()?;
        // Example JiraIssue data
        let issues = vec![
            JiraIssue {
                id: "1".to_string(),
                self_url: "https://example.com/issue/1".to_string(),
                key: JiraKey::new("ISSUE-1"),
                worklogs: vec![],
                fields: JiraFields {
                    summary: "This is the first issue.".to_string(),
                    asset: None
                },
            },
            JiraIssue {
                id: "2".to_string(),
                self_url: "https://example.com/issue/2".to_string(),
                key: JiraKey::new("ISSUE-2"),
                worklogs: vec![],
                fields: JiraFields {
                    summary: "This is the second issue.".to_string(),
                    asset: None
                },
            },
        ];
        let _result = lws.add_jira_issues(&issues)?;
        let issues = lws.get_jira_issues_filtered_by_keys(vec![JiraKey::from("ISSUE-1"), JiraKey::from("Issue-2")])?;
        assert_eq!(issues.len(), 2);

        Ok(())
    }
}
