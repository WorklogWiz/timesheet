use crate::error::WorklogError;
use jira::models::user::User;

/// Placeholder for the User repository, i.e. all CRUD operations related to the `User` entity.
#[allow(dead_code)]
pub trait UserRepository {
    fn insert_or_update_current_user(&self, user: &User) -> Result<(), WorklogError>;
    fn find_user(&self) -> Result<User, WorklogError>;
}
