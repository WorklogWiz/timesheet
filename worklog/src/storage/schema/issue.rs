const CREATE_ISSUE_TABLE_SQL: &str = r"
    CREATE TABLE IF NOT EXISTS issue (
        issue_key varchar(32) primary key,
        summary varchar(1024) not null
    );
";

pub fn create_issue_table(conn: &rusqlite::Connection) -> Result<(), rusqlite::Error> {
    conn.execute(CREATE_ISSUE_TABLE_SQL, [])?;
    Ok(())
}
