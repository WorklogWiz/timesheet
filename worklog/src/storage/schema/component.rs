const CREATE_COMPONENT_TABLE_SQL: &str = r"
    CREATE TABLE IF NOT EXISTS component (
        id integer primary key not null,
        name varchar(1024) not null
    );
";

pub fn create_component_table(conn: &rusqlite::Connection) -> Result<(), rusqlite::Error> {
    conn.execute(CREATE_COMPONENT_TABLE_SQL, [])?;
    Ok(())
}