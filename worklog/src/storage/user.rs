use crate::error::WorklogError;
use crate::storage::dbms::Dbms;
use jira::models::user::User;
use rusqlite::params;

impl Dbms {
    /// Implements methods for interacting with the database.
    ///
    /// The `Dbms` struct provides functions for inserting, updating, and retrieving
    /// user records from the database. These methods handle database interactions, including
    /// error mapping into the custom `WorklogError` type.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let dbms = Dbms::new(connection);
    /// let user = User {
    ///     account_id: "12345".to_string(),
    ///     email_address: "user@example.com".to_string(),
    ///     display_name: "Example User".to_string(),
    ///     time_zone: "UTC".to_string(),
    ///     self_url: "https://jira.example.com/user/12345".to_string(),
    /// };
    ///
    /// // Insert or update the user
    /// dbms.insert_or_update_current_user(&user)?;
    ///
    /// // Retrieve the user
    /// let retrieved_user = dbms.find_user()?;
    /// assert_eq!(retrieved_user.account_id, user.account_id);
    /// ```
    ///
    /// # Errors
    ///
    /// This function will return a `WorklogError` if:
    /// * There is an SQL syntax issue or constraint violation while executing the `INSERT OR IGNORE` statement.
    /// * A problem occurs while preparing or executing the SQL statement.
    /// * There is a database connection failure or underlying database error.
    pub fn insert_or_update_current_user(&self, user: &User) -> Result<(), WorklogError> {
        let sql = "INSERT OR IGNORE INTO user (account_id, email, display_name, timezone) VALUES (?, ?, ?, ?)";
        let mut stmt = self.connection.prepare(sql)?;
        stmt.execute(params![
            user.account_id,
            user.email_address,
            user.display_name,
            user.time_zone
        ])
        .map_err(|e| WorklogError::Sql(format!("Unable to insert user {user:?}: {e}")))?;
        Ok(())
    }

    /// Updates or inserts a user into the database.
    ///
    /// This method tries to insert a user's information into the `user` table. If the user already exists
    /// (based on the primary key or unique constraints), the existing record is preserved with no changes.
    ///
    /// # Arguments
    ///
    /// * `user` - A reference to a `User` object containing the user's information to be inserted or updated.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the operation succeeds without any errors.
    /// * `Err(WorklogError)` - If an error occurs while interacting with the database.
    ///
    /// # Errors
    ///
    /// This function will return a `WorklogError` if:
    /// * There is an SQL syntax issue or database constraint violation.
    /// * A database connection issue occurs.
    pub fn find_user(&self) -> Result<User, WorklogError> {
        let sql = "select account_id, email, display_name, timezone from user";
        let mut stmt = self.connection.prepare(sql)?;
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
    use crate::storage::dbms::tests::setup;
    #[test]
    fn test_add_user() -> Result<(), WorklogError> {
        let lws = setup()?;
        let user = User {
            account_id: "712020:719b6d98-78c7-4c63-a564-299916c67765".to_string(),
            email_address: "steinar@gastroplanner.no".to_string(),
            display_name: "Steinar Overbeck Cook".to_string(),
            time_zone: "Europe/Oslo".to_string(),
            self_url: "https://xxxxxxxx.atlassian.net/rest/api/2/user?accountId=713020:719b6d98-78c7-4c63-a564-299916c67765".to_string()
        };
        lws.insert_or_update_current_user(&user)?;
        let dbms_usr = lws.find_user()?;
        assert_eq!(dbms_usr.account_id, user.account_id);

        Ok(())
    }
}
