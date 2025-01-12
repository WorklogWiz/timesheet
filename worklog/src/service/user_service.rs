use crate::error::WorklogError;
use crate::repository::user_repository::UserRepository;
use jira::models::user::User;
use std::sync::Arc;

pub struct UserService<R: UserRepository> {
    repo: Arc<R>,
}

impl<R: UserRepository> UserService<R> {
    pub fn new(repo: Arc<R>) -> Self {
        Self { repo }
    }

    pub fn insert_or_update_current_user(&self, user: &User) -> Result<(), WorklogError> {
        self.repo.insert_or_update_current_user(user)
    }

    pub fn find_user(&self) -> Result<User, WorklogError> {
        self.repo.find_user()
    }
}
