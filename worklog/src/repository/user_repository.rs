//! This module defines the `UserRepository` trait for interacting with user data in a repository.
//!
//! The trait allows for inserting or updating a user in the repository as well as retrieving the current user.
//!
//! # Traits
//!
//! The main trait [`UserRepository`] provides methods for:
//! - Inserting or updating the current user via [`UserRepository::insert_or_update_current_user`].
//! - Retrieving the current user via [`UserRepository::find_user`].
//!
//! # Errors
//!
//! The trait methods use the [`WorklogError`] type to represent potential errors during the operations.
use crate::error::WorklogError;
use jira::models::user::User;

#[allow(dead_code)]
pub trait UserRepository: Send + Sync {
    /// Inserts or updates the current user in the repository.
    ///
    /// # Arguments
    ///
    /// * `user` - A reference to the [User] entity that will be inserted or updated.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the operation was successful.
    /// * `Err(WorklogError)` - If there was an issue during the operation.
    fn insert_or_update_current_user(&self, user: &User) -> Result<(), WorklogError>;

    /// Finds and retrieves the current user from the repository.
    ///
    /// # Returns
    ///
    /// * `Ok(User)` - If the user was found successfully.
    /// * `Err(WorklogError)` - If there was an issue, such as the user not being found.
    fn find_user(&self) -> Result<User, WorklogError>;
}
