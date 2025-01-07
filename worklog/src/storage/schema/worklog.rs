use anyhow::Result;
use rusqlite::Connection;

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
pub fn create_worklog_table(connection: &Connection) -> Result<()> {
    connection.execute(CREATE_WORKLOG_TABLE_SQL, [])?;
    Ok(())
}
