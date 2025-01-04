use crate::error::WorklogError;
use chrono::{DateTime, Local};
use jira::models::issue::IssueSummary;
use jira::models::project::Component;
use jira::models::user::User;
use jira::models::{core::IssueKey, worklog::Worklog};
use log::debug;
use rusqlite::{named_params, params, Connection};
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, PartialOrd, Ord, Clone)]
#[allow(non_snake_case)]
#[allow(clippy::module_name_repetitions)]
pub struct LocalWorklog {
    pub issue_key: IssueKey,
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

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, Clone)]
pub struct JiraIssueInfo {
    pub issue_key: IssueKey,
    pub summary: String,
}

impl LocalWorklog {
    /// Converts a Jira `Worklog` entry into a `LocalWorklog` entry
    #[must_use]
    pub fn from_worklog(worklog: &Worklog, issue_key: &IssueKey) -> Self {
        LocalWorklog {
            issue_key: issue_key.clone(),
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

#[allow(clippy::module_name_repetitions)]
pub struct WorklogStorage {
    connection: Connection,
}

impl WorklogStorage {
    ///
    /// Adds a list of components to the local database and associates them with the given issue key.
    ///
    /// This function inserts the provided `components` into the `component` table and ensures that
    /// they are linked with the specified `issue_key` in the `issue_component` table.
    /// If a component with the same ID already exists, it updates its name.
    /// The `issue_key` and component IDs are also added to the `issue_component` table, avoiding duplicates.
    ///
    /// # Arguments
    /// * `issue_key` - The issue key to associate the components with.
    /// * `components` - A list of `Component` objects to add to the database.
    ///
    /// # Errors
    /// Returns a `WorklogError` if any SQL operation fails during the insertion or association.
    ///
    /// # Panics
    /// This method panics if it encounters any error during the execution of the SQL statements.
    pub fn add_component(
        &self,
        issue_key: &IssueKey,
        components: &Vec<Component>,
    ) -> Result<(), WorklogError> {
        let mut insert_component_stmt = self.connection.prepare(
            "INSERT INTO component (id, name)
            VALUES (?1, ?2)
            ON CONFLICT(id) DO UPDATE SET name = excluded.name",
        )?;

        debug!("Adding components for issue {issue_key}");
        for component in components {
            debug!("Adding component id {} for issue {issue_key}", component.id);
            // Consider using the return value to count number of rows that were actually
            // inserted
            if let Err(e) =
                insert_component_stmt.execute(params![component.id, component.name.clone()])
            {
                panic!(
                    "Unable to insert component({},{}): {}",
                    component.id, component.name, e
                );
            }
        }
        // Links the components with the issues to maintain the many-to-many relationship
        let mut insert_issue_component_stmt = self.connection.prepare(
            "INSERT OR IGNORE INTO issue_component (issue_key, component_id) VALUES (?1, ?2)",
        )?;
        for component in components {
            debug!(
                "Adding issue_component ({}, {})",
                issue_key.value, component.id
            );
            if let Err(e) =
                insert_issue_component_stmt.execute(params![issue_key.value(), component.id])
            {
                panic!(
                    "Unable to insert issue_component({},{}): {}",
                    issue_key.value(),
                    component.id,
                    e
                );
            }
        }
        Ok(())
    }

    ///
    /// Adds multiple Jira issues to the local database.
    ///
    /// This function inserts Jira issues into the `issue` table of the local database.
    /// If an issue with the same `issue_key` already exists, its `summary` is updated.
    ///
    /// # Arguments
    ///
    /// * `jira_issues` - A vector of `IssueSummary` objects to be added to the database.
    ///
    /// # Errors
    ///
    /// Returns a `WorklogError` if any SQL operation fails during the insertion or update.
    ///
    /// # Panics
    ///
    /// This method panics if any SQL statement execution fails due to unexpected conditions.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let issues = vec![
    ///     IssueSummary { key: IssueKey::new("ISSUE-1"), fields: Fields { summary: "Issue 1".to_string() } },
    ///     IssueSummary { key: IssueKey::new("ISSUE-2"), fields: Fields { summary: "Issue 2".to_string() } },
    /// ];
    ///
    /// worklog_storage.add_jira_issues(&issues)?;
    /// ```
    pub fn add_jira_issues(&self, jira_issues: &Vec<IssueSummary>) -> Result<(), WorklogError> {
        let mut stmt = self.connection.prepare(
            "INSERT INTO issue (issue_key, summary)
            VALUES (?1, ?2)
            ON CONFLICT(issue_key) DO UPDATE SET summary = excluded.summary",
        )?;
        for issue in jira_issues {
            if let Err(e) = stmt.execute(params![issue.key.to_string(), issue.fields.summary]) {
                panic!(
                    "Unable to insert issue({},{}): {}",
                    issue.key, issue.fields.summary, e
                );
            }
        }
        Ok(())
    }

    ///
    /// Retrieves a list of issues from the database filtered by the provided issue keys.
    ///
    /// This function queries the local database for issues whose keys match those
    /// provided in the `keys` parameter. It dynamically constructs the SQL query
    /// to handle a variable number of keys using placeholders. If no keys are provided,
    /// it will return an empty vector.
    ///
    /// # Arguments
    ///
    /// * `keys` - A vector of issue keys of type `IssueKey` to filter the issues.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing a vector of `JiraIssueInfo` objects representing the
    /// matching issues. If an error occurs while querying the database, a `WorklogError` is returned.
    ///
    /// # Errors
    ///
    /// This function may return a `WorklogError` if an error occurs while preparing or
    /// executing the SQL statement, or while processing the result rows.
    ///
    /// # Examples
    ///
    /// ```rust,ignore  
    /// let issue_keys = vec![IssueKey::new("ISSUE-1"), IssueKey::new("ISSUE-2")];
    /// let issues = worklog_storage.get_issues_filtered_by_keys(&issue_keys)?;
    ///
    /// for issue in issues {
    ///     println!("Issue Key: {}, Summary: {}", issue.issue_key.value(), issue.summary);
    /// }
    /// ```
    ///
    pub fn get_issues_filtered_by_keys(
        &self,
        keys: &Vec<IssueKey>,
    ) -> Result<Vec<JiraIssueInfo>, WorklogError> {
        if keys.is_empty() {
            // Return an empty vector if no keys are provided
            return Ok(Vec::new());
        }

        debug!("selecting issue from database for keys {:?}", keys);

        // Build the `IN` clause dynamically
        let placeholders = keys.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
        let sql = format!(
            "SELECT issue_key, summary
            FROM issue
            WHERE issue_key IN ({placeholders})"
        );

        // Prepare the parameters for the query
        let params: Vec<String> = keys.iter().map(ToString::to_string).collect();

        let mut stmt = self.connection.prepare(&sql)?;

        let issues = stmt
            .query_map(rusqlite::params_from_iter(params), |row| {
                Ok(JiraIssueInfo {
                    issue_key: IssueKey::new(&row.get::<_, String>(0)?),
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
            .execute("DELETE FROM worklog WHERE id = ?1", params![wl_id])?;
        Ok(())
    }

    ///
    /// # Errors
    /// Returns an error something goes wrong
    pub fn new(dbms_path: &Path) -> Result<Self, WorklogError> {
        if let Some(parent) = dbms_path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)?;
            }
        }

        let connection = Connection::open(dbms_path)?;

        connection.path();
        // Creates the schema if needed
        create_local_worklog_schema(&connection)?;

        Ok(WorklogStorage { connection })
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

    /// Retrieves the path to the current open database
    pub fn get_dbms_path(&self) -> &str {
        self.connection.path().unwrap_or_default()
    }

    ///
    /// # Errors
    /// Returns an error something goes wrong
    pub fn find_unique_keys(&self) -> Result<Vec<IssueKey>, WorklogError> {
        let mut stmt = self
            .connection
            .prepare("SELECT DISTINCT(issue_key) FROM worklog ORDER BY issue_key asc")?;
        let issue_keys: Vec<IssueKey> = stmt
            .query_map([], |row| {
                let key: String = row.get::<_, String>(0)?;
                Ok(IssueKey::from(key))
            })?
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

///
/// # Errors
/// Returns an error something goes wrong
pub fn create_local_worklog_schema(connection: &Connection) -> Result<(), WorklogError> {
    let sql = r"
        CREATE TABLE IF NOT EXISTS issue (
            issue_key varchar(32) primary key,
            summary varchar(1024) not null
        );
    ";
    connection
        .execute(sql, [])
        .map_err(|e| WorklogError::Sql(format!("Unable to create table 'issue': {e}")))?;

    let sql = r"
        CREATE TABLE IF NOT EXISTS worklog (
            id integer primary key not null,
            issue_key varchar(32),
            issue_id varchar(32),
            author varchar(1024),
            created datetime,
            updated datetime,
            started datetime,
            time_spent varchar(32),
            time_spent_seconds intger,
            comment varchar(1024),
            FOREIGN KEY (issue_key) REFERENCES issue(issue_key) ON DELETE CASCADE
        );
    ";
    connection
        .execute(sql, [])
        .map_err(|e| WorklogError::Sql(format!("Unable to create table 'worklog': {e}")))?;

    let sql = r"
        create table if not exists component (
            id integer primary key not null,
            name varchar(1024) not null
        );
    ";
    connection
        .execute(sql, [])
        .map_err(|e| WorklogError::Sql(format!("Unable to create table 'component': {e}")))?;

    let sql = r"
    CREATE TABLE if not exists issue_component (
        id INTEGER PRIMARY KEY NOT NULL,
        issue_key VARCHAR(32) NOT NULL,
        component_id INTEGER NOT NULL,
        FOREIGN KEY (issue_key) REFERENCES issue(issue_key) ON DELETE CASCADE,
        FOREIGN KEY (component_id) REFERENCES component(id) ON DELETE CASCADE,
        UNIQUE(issue_key, component_id)
    );
    ";
    connection
        .execute(sql, [])
        .map_err(|e| WorklogError::Sql(format!("Unable to create table 'issue_component': {e}")))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Days, Local};
    use jira::models::core::Fields;

    use rusqlite::Connection;

    fn setup_in_memory_db() -> Result<Connection, WorklogError> {
        let conn = Connection::open_in_memory()?;
        create_local_worklog_schema(&conn)?;
        Ok(conn)
    }

    pub fn setup() -> Result<WorklogStorage, WorklogError> {
        let lws = WorklogStorage {
            connection: setup_in_memory_db()?,
        };
        Ok(lws)
    }

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

    #[test]
    fn add_issues() -> Result<(), WorklogError> {
        let lws = setup()?;
        // Example JiraIssue data
        let issues = vec![
            IssueSummary {
                id: "1".to_string(),
                key: IssueKey::new("ISSUE-1"),
                fields: Fields {
                    summary: "This is the first issue.".to_string(),
                    components: vec![],
                },
            },
            IssueSummary {
                id: "2".to_string(),
                key: IssueKey::new("ISSUE-2"),
                fields: Fields {
                    summary: "This is the second issue.".to_string(),
                    components: vec![],
                },
            },
        ];
        lws.add_jira_issues(&issues)?;
        let issues = lws.get_issues_filtered_by_keys(&vec![
            IssueKey::from("ISSUE-1"),
            IssueKey::from("Issue-2"),
        ])?;
        assert_eq!(issues.len(), 2);

        Ok(())
    }
}
