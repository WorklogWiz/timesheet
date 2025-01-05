use rusqlite::params;
use jira::models::user::User;
use crate::error::WorklogError;
use crate::storage::dbms_repository::DbmsRepository;

impl DbmsRepository {
    pub fn insert_or_update_current_user(&self, user: &User) -> Result<(), WorklogError> {
        let sql = "INSERT OR IGNORE INTO user (account_id, email, display_name, timezone) VALUES (?, ?, ?, ?)";
        let mut stmt = self.connection.prepare(sql)?;
        stmt.execute(params![user.account_id, user.email_address, user.display_name, user.time_zone])
            .map_err(|e| WorklogError::Sql(format!("Unable to insert user {:?}: {}", user, e)))?;
        Ok(())
    }

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

        let user = user_iter.next().transpose()?
            .ok_or_else(|| WorklogError::Sql("No user found".to_string()))?;
        Ok(user)
    }

}

#[cfg(test)]
mod tests {
    use crate::storage::dbms_repository::tests::setup;
    use super::*;
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