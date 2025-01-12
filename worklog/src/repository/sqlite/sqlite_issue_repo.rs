use crate::error::WorklogError;
use crate::repository::issue_repository::IssueRepository;
use crate::types::JiraIssueInfo;
use jira::models::core::IssueKey;
use jira::models::issue::IssueSummary;
use log::debug;
use rusqlite::{params, Connection};
use std::sync::{Arc, Mutex};

pub struct SqliteIssueRepository {
    connection: Arc<Mutex<Connection>>,
}

impl SqliteIssueRepository {
    pub fn new(connection: Arc<Mutex<Connection>>) -> Self {
        Self { connection }
    }
}

const CREATE_ISSUE_TABLE_SQL: &str = r"
    CREATE TABLE IF NOT EXISTS issue (
        id integer primary key,
        key varchar(32) not null unique,
        summary varchar(1024) not null
    );
";

pub(crate) fn create_issue_table(conn: Arc<Mutex<Connection>>) -> Result<(), rusqlite::Error> {
    let conn = conn.lock().unwrap();
    conn.execute(CREATE_ISSUE_TABLE_SQL, [])?;
    Ok(())
}

impl IssueRepository for SqliteIssueRepository {
    ///
    /// Adds multiple Jira issues to the local database.
    ///
    /// This function inserts Jira issues into the `issue` table of the local database.
    /// If an issue with the same `key` already exists, its `summary` is updated.
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
    fn add_jira_issues(&self, jira_issues: &Vec<IssueSummary>) -> Result<(), WorklogError> {
        let conn = self.connection.lock().unwrap();
        let mut stmt = conn.prepare(
            "INSERT INTO issue (id, key, summary)
            VALUES (?1, ?2, ?3)
            ON CONFLICT(id) DO UPDATE SET summary = excluded.summary, key = excluded.key",
        )?;
        for issue in jira_issues {
            if let Err(e) = stmt.execute(params![
                issue.id,
                issue.key.to_string(),
                issue.fields.summary
            ]) {
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
    ///     println!("Issue Key: {}, Summary: {}", issue.key.value(), issue.summary);
    /// }
    /// ```
    ///
    fn get_issues_filtered_by_keys(
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
            "SELECT key, summary
            FROM issue
            WHERE key IN ({placeholders})"
        );

        // Prepare the parameters for the query
        let params: Vec<String> = keys.iter().map(ToString::to_string).collect();
        let conn = self.connection.lock().unwrap();
        let mut stmt = conn.prepare(&sql)?;

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
    fn find_unique_keys(&self) -> Result<Vec<IssueKey>, WorklogError> {
        let conn = self.connection.lock().unwrap();
        let mut stmt = conn.prepare("SELECT DISTINCT(key) FROM worklog ORDER BY key asc")?;
        let issue_keys: Vec<IssueKey> = stmt
            .query_map([], |row| {
                let key: String = row.get::<_, String>(0)?;
                Ok(IssueKey::from(key))
            })?
            .filter_map(Result::ok)
            .collect();
        Ok(issue_keys)
    }
}
