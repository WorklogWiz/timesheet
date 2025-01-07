const CREATE_ISSUE_COMPONENT_TABLE_SQL: &str = r"
    -- Association between the tables issue and component
    CREATE TABLE IF NOT EXISTS issue_component (
        id INTEGER PRIMARY KEY NOT NULL,
        key VARCHAR(32) NOT NULL,
        component_id INTEGER NOT NULL,
        FOREIGN KEY (key) REFERENCES issue(key) ON DELETE CASCADE,
        FOREIGN KEY (component_id) REFERENCES component(id) ON DELETE CASCADE,
        UNIQUE(key, component_id)
    );
";

pub fn create_issue_component_table(conn: &rusqlite::Connection) -> Result<(), rusqlite::Error> {
    conn.execute(CREATE_ISSUE_COMPONENT_TABLE_SQL, [])?;
    Ok(())
}
