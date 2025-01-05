use log::debug;
use rusqlite::params;
use jira::models::core::IssueKey;
use jira::models::issue::IssueSummary;
use crate::error::WorklogError;
use crate::types::JiraIssueInfo;
use crate::storage::dbms_repository::DbmsRepository;

impl DbmsRepository {

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

}

#[cfg(test)]
mod tests {
    use jira::models::core::Fields;
    use crate::storage::dbms_repository::tests::setup;
    use super::*;
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