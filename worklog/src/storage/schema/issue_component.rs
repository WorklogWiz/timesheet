const CREATE_ISSUE_COMPONENT_TABLE_SQL: &str = r"
    -- Association between the tables issue and component
    CREATE TABLE IF NOT EXISTS issue_component (
        id INTEGER PRIMARY KEY NOT NULL,
        issue_key VARCHAR(32) NOT NULL,
        component_id INTEGER NOT NULL,
        FOREIGN KEY (issue_key) REFERENCES issue(issue_key) ON DELETE CASCADE,
        FOREIGN KEY (component_id) REFERENCES component(id) ON DELETE CASCADE,
        UNIQUE(issue_key, component_id)
    );
";

pub fn create_issue_component_table(conn: &rusqlite::Connection) -> Result<(), rusqlite::Error> {
    conn.execute(CREATE_ISSUE_COMPONENT_TABLE_SQL, [])?;
    Ok(())
}