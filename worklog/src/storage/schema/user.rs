use rusqlite::Connection;
use anyhow::Result;

/// SQL statement to create the `user` table.
const CREATE_USER_TABLE_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS user (
    account_id varchar(128) primary key NOT NULL,
    email varchar(1024) unique,
    display_name varchar(512) NOT NULL,
    timezone varchar(64) NOT NULL
);
"#;

/// Creates the `user` table in the database.
pub fn create_user_table(connection: &Connection) -> Result<()> {
    connection.execute(CREATE_USER_TABLE_SQL, [])?;
    Ok(())
}