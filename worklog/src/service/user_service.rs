/// A service responsible for managing user-related operations.
///
/// The `UserService` acts as an intermediary between the user repository
/// and the application logic, providing functionality to insert, update,
/// or retrieve user information from the data source.
use crate::error::WorklogError;
use crate::repository::user_repository::UserRepository;
use jira::models::user::User;
use std::sync::Arc;

pub struct UserService {
    repo: Arc<dyn UserRepository>,
}

impl UserService {
    pub fn new(repo: Arc<dyn UserRepository>) -> Self {
        Self { repo }
    }

    /// Inserts or updates the current user's information in the repository.
    ///
    /// If the user already exists in the data source, the method updates the
    /// user's information. Otherwise, it inserts the new user's information
    /// into the data source.
    ///
    /// # Arguments
    ///
    /// * `user` - A reference to the `User` object containing the user's details
    ///            to be inserted or updated in the repository.
    ///
    /// # Errors
    ///
    /// Returns a `WorklogError` if the operation fails due to an issue with
    /// the repository or data source.
    pub fn insert_or_update_current_user(&self, user: &User) -> Result<(), WorklogError> {
        self.repo.insert_or_update_current_user(user)
    }

    /// Finds and retrieves the current user's information from the repository.
    ///
    /// This method queries the data source through the user repository to fetch the
    /// details of the current user stored in the system.
    ///
    /// # Returns
    ///
    /// * `Ok(User)` - The `User` object containing the details of the current user
    ///   if found in the data source.
    /// * `Err(WorklogError)` - An error if the operation fails due to an issue with
    ///   the repository or if the user cannot be found.
    ///
    /// # Errors
    ///
    /// This method will return a `WorklogError` if:
    /// - There is a problem with the repository or data source.
    /// - The user information cannot be retrieved.
    pub fn find_current_user(&self) -> Result<User, WorklogError> {
        self.repo.find_user()
    }
}
