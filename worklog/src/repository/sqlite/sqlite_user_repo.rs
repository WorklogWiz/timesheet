use crate::error::WorklogError;
use crate::repository::user_repository::UserRepository;
use jira::models::user::User;
use rusqlite::{params, Connection};
use std::sync::{Arc, Mutex};

pub struct SqliteUserRepository {
    connection: Arc<Mutex<Connection>>,
}

impl SqliteUserRepository {
    pub fn new(connection: Arc<Mutex<Connection>>) -> Self {
        Self { connection }
    }
}

/// SQL statement to create the `user` table.
const CREATE_USER_TABLE_SQL: &str = r"
CREATE TABLE IF NOT EXISTS user (
    account_id varchar(128) primary key NOT NULL,
    email varchar(1024) unique,
    display_name varchar(512) NOT NULL,
    timezone varchar(64) NOT NULL
);
";

/// Creates the `user` table in the database.
pub(crate) fn create_schema(connection: Arc<Mutex<Connection>>) -> Result<(), WorklogError> {
    let conn = connection.lock().map_err(|_| WorklogError::LockPoisoned)?;
    conn.execute(CREATE_USER_TABLE_SQL, [])?;
    Ok(())
}
impl UserRepository for SqliteUserRepository {
    fn insert_or_update_current_user(&self, user: &User) -> Result<(), WorklogError> {
        let sql = "INSERT OR IGNORE INTO user (account_id, email, display_name, timezone) VALUES (?, ?, ?, ?)";
        let conn = self.connection.lock().unwrap();
        let mut stmt = conn.prepare(sql)?;
        stmt.execute(params![
            user.account_id,
            user.email_address,
            user.display_name,
            user.time_zone
        ])
        .map_err(|e| WorklogError::Sql(format!("Unable to insert user {user:?}: {e}")))?;
        Ok(())
    }

    fn find_user(&self) -> Result<User, WorklogError> {
        let sql = "select account_id, email, display_name, timezone from user";
        let conn = self.connection.lock().unwrap();
        let mut stmt = conn.prepare(sql)?;
        let mut user_iter = stmt.query_map([], |row| {
            Ok(User {
                account_id: row.get(0)?,
                email_address: row.get(1)?,
                display_name: row.get(2)?,
                time_zone: row.get(3)?,
                ..Default::default()
            })
        })?;

        let user = user_iter
            .next()
            .transpose()?
            .ok_or_else(|| WorklogError::Sql("No user found".to_string()))?;
        Ok(user)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::sqlite::tests::test_database_manager;
    #[test]
    fn test_add_user() -> Result<(), WorklogError> {
        let db_manager = test_database_manager()?;
        let user = User {
            account_id: "712020:719b6d98-78c7-4c63-a564-299916c67765".to_string(),
            email_address: "steinar@gastroplanner.no".to_string(),
            display_name: "Steinar Overbeck Cook".to_string(),
            time_zone: "Europe/Oslo".to_string(),
            self_url: "https://xxxxxxxx.atlassian.net/rest/api/2/user?accountId=713020:719b6d98-78c7-4c63-a564-299916c67765".to_string()
        };
        let user_repo = db_manager.create_user_repository();
        user_repo.insert_or_update_current_user(&user)?;
        let dbms_usr = user_repo.find_user()?;
        assert_eq!(dbms_usr.account_id, user.account_id);

        Ok(())
    }
}
