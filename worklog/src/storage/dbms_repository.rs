use std::fs;
use std::path::Path;
use rusqlite::Connection;
use crate::error::WorklogError;
use crate::storage::schema::create_schema;

pub struct DbmsRepository {
    pub(crate) connection: Connection,
}

impl DbmsRepository {
    ///
    /// Creates a new instance of `DbmsRepository` by opening or creating the specified SQLite database.
    ///
    /// This function ensures that the parent directories of the provided database path exist,
    /// creating them if necessary, and initializes the database schema.
    ///
    /// # Arguments
    ///
    /// * `dbms_path` - The file path to the SQLite database.
    ///
    /// # Returns
    ///
    /// A `Result` containing the newly created `DbmsRepository` instance
    /// or a `WorklogError` if there is an issue opening the database or creating the schema.
    ///
    /// # Errors
    ///
    /// This function will return an error if the database cannot be opened, the schema
    /// cannot be created, or the directory creation fails.
    pub fn new(dbms_path: &Path) -> Result<Self, WorklogError> {
        if let Some(parent) = dbms_path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)?;
            }
        }

        let connection = Connection::open(dbms_path)?;

        connection.path();
        // Creates the schema if needed
        create_schema(&connection)?;

        Ok(DbmsRepository { connection })
    }

    /// Retrieves the path to the current open database
    pub fn get_dbms_path(&self) -> &str {
        self.connection.path().unwrap_or_default()
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;


    fn setup_in_memory_db() -> Result<Connection, WorklogError> {
        let conn = Connection::open_in_memory()?;
        crate::storage::schema::create_schema(&conn)?;
        Ok(conn)
    }

    pub fn setup() -> Result<crate::storage::dbms_repository::DbmsRepository, WorklogError> {
        let lws = crate::storage::dbms_repository::DbmsRepository {
            connection: setup_in_memory_db()?,
        };
        Ok(lws)
    }

}
