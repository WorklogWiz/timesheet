use std::path::PathBuf;
use chrono::{DateTime, Local, Utc};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use serde_rusqlite::to_params_named;
use common::WorklogError;
use jira_lib::{Author, JiraKey, Worklog};

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, PartialOrd)]
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

pub struct LocalWorklogService {
    dbms_path: PathBuf,
    connection: Connection,
}

impl LocalWorklogService {
    pub fn new(dbms_path: &PathBuf) -> Result<Self, WorklogError> {
        let connection = Connection::open(dbms_path.as_path())
            .map_err(|e| WorklogError::OpenDbms {path: dbms_path.to_string_lossy().into(), reason: e.to_string()})?;
        // Creates the schema if needed
        create_local_worklog_schema(&connection)?;

        Ok(LocalWorklogService{ connection, dbms_path: dbms_path.to_path_buf()})
    }

    /// Adds a new entry into the local DBMS
    pub fn add_entry(&self, local_worklog: LocalWorklog) -> Result<(), WorklogError> {
        let i = self.connection.execute(
            "INSERT INTO worklog (
            issue_key, id, author, created, updated, started, time_Spent, time_Spent_Seconds, issue_Id, comment
        ) VALUES (
            :issue_key, :id, :author, :created, :updated, :started, :timeSpent, :timeSpentSeconds, :issueId, :comment
        )",
            to_params_named(&local_worklog).map_err(|e| WorklogError::Sql(format!("Unable to convert parameters: {}", e.to_string())))?.to_slice().as_slice(),
        ).map_err(|e| WorklogError::Sql(format!("Unable to insert into worklog: {}", e.to_string())))?;
        if i ==0 {
            panic!("Insert failed");
        }
        Ok(())
    }


}

pub fn create_local_worklog_schema(connection: &Connection) -> Result<(), WorklogError> {
    let sql = r#"
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
    "#;
    let result = connection.execute(&sql, [])
        .map_err(|e| WorklogError::Sql(format!("Unable to create table 'worklog': {}", e.to_string())))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use chrono::Local;
    use common::config;
    use super::*;

    pub fn setup() -> Result<LocalWorklogService, WorklogError> {
        let tmp_db = config::tmp_local_worklog_dbms_file_name()
            .map_err(|e| WorklogError::CreateFile("temp file".to_string()))?;

        let local_worklog_service = LocalWorklogService::new(&tmp_db)?;
        create_local_worklog_schema(&local_worklog_service.connection)?;
        Ok(local_worklog_service)
    }

    #[test]
    fn test_open_dbms() -> Result<(), WorklogError>{
        let local_worklog_service = setup()?;
        Ok(())
    }

    #[test]
    fn test_add_local_worklog_entry() -> Result<(), WorklogError>{
        let worklog = LocalWorklog {
            issue_key: JiraKey("ABC-123".to_string()),
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
        worklog_service.add_entry(worklog)?;

        Ok(())
    }
}
