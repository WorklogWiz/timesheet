use crate::domain::User;
/// Service for managing user-related operations
///
/// The `UserService` acts as an intermediary between the user repository
/// and the application logic, providing functionality to fetch user information
/// from the remote tracker and cache it locally.
use crate::error::WorklogError;
use crate::traits::{IssueTrackerClient, UserRepository};
use std::sync::Arc;

#[allow(clippy::module_name_repetitions)]
pub struct UserService {
    repo: Arc<dyn UserRepository>,
    tracker: Arc<dyn IssueTrackerClient>,
}

impl UserService {
    pub fn new(repo: Arc<dyn UserRepository>, tracker: Arc<dyn IssueTrackerClient>) -> Self {
        Self { repo, tracker }
    }

    /// Ensures the current user is fetched from the tracker and saved locally
    ///
    /// This method fetches the current user from the remote issue tracker
    /// and caches it in the local database. This is useful for initializing
    /// the user context before operations that require user information.
    ///
    /// # Returns
    ///
    /// The current user information
    ///
    /// # Errors
    ///
    /// Returns `WorklogError` if fetching from tracker or saving to repo fails
    pub async fn ensure_current_user(&self) -> Result<User, WorklogError> {
        let user = self.tracker.get_current_user().await?;
        self.repo.save_current_user(&user).await?;
        Ok(user)
    }

    /// Retrieves the current user's information from the local repository
    ///
    /// # Returns
    ///
    /// The current user if found, None otherwise
    ///
    /// # Errors
    ///
    /// Returns `WorklogError` if the database operation fails
    pub async fn get_current_user(&self) -> Result<Option<User>, WorklogError> {
        self.repo.get_current_user().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::traits::{MockIssueTrackerClient, MockUserRepository};

    fn create_test_user(id: &str, display_name: &str) -> User {
        User {
            id: id.to_string(),
            display_name: display_name.to_string(),
            email: Some(format!("{display_name}@example.com")),
            timezone: Some("UTC".to_string()),
        }
    }

    #[tokio::test]
    async fn test_ensure_current_user_fetches_and_saves() {
        let test_user = create_test_user("user-123", "johndoe");
        let user_clone = test_user.clone();

        let mut mock_repo = MockUserRepository::new();
        mock_repo
            .expect_save_current_user()
            .withf(move |u| u.id == user_clone.id)
            .times(1)
            .returning(|_| Ok(()));

        let mut mock_tracker = MockIssueTrackerClient::new();
        mock_tracker
            .expect_get_current_user()
            .times(1)
            .returning(move || Ok(test_user.clone()));

        let service = UserService::new(Arc::new(mock_repo), Arc::new(mock_tracker));

        let result = service.ensure_current_user().await.unwrap();

        assert_eq!(result.id, "user-123");
        assert_eq!(result.display_name, "johndoe");
        assert_eq!(result.email, Some("johndoe@example.com".to_string()));
    }

    #[tokio::test]
    async fn test_ensure_current_user_propagates_tracker_error() {
        let mut mock_repo = MockUserRepository::new();
        mock_repo.expect_save_current_user().times(0);

        let mut mock_tracker = MockIssueTrackerClient::new();
        mock_tracker
            .expect_get_current_user()
            .times(1)
            .returning(|| {
                Err(WorklogError::IssueTrackerError(
                    "Authentication failed".to_string(),
                ))
            });

        let service = UserService::new(Arc::new(mock_repo), Arc::new(mock_tracker));

        let result = service.ensure_current_user().await;

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Authentication failed"));
    }

    #[tokio::test]
    async fn test_ensure_current_user_propagates_repo_error() {
        let test_user = create_test_user("user-123", "johndoe");

        let mut mock_repo = MockUserRepository::new();
        mock_repo
            .expect_save_current_user()
            .times(1)
            .returning(|_| Err(WorklogError::StorageError("Database locked".to_string())));

        let mut mock_tracker = MockIssueTrackerClient::new();
        mock_tracker
            .expect_get_current_user()
            .times(1)
            .returning(move || Ok(test_user.clone()));

        let service = UserService::new(Arc::new(mock_repo), Arc::new(mock_tracker));

        let result = service.ensure_current_user().await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Database locked"));
    }

    #[tokio::test]
    async fn test_get_current_user_returns_cached_user() {
        let test_user = create_test_user("user-456", "janedoe");
        let user_clone = test_user.clone();

        let mut mock_repo = MockUserRepository::new();
        mock_repo
            .expect_get_current_user()
            .times(1)
            .returning(move || Ok(Some(user_clone.clone())));

        let mock_tracker = MockIssueTrackerClient::new();

        let service = UserService::new(Arc::new(mock_repo), Arc::new(mock_tracker));

        let result = service.get_current_user().await.unwrap();

        assert!(result.is_some());
        let user = result.unwrap();
        assert_eq!(user.id, "user-456");
        assert_eq!(user.display_name, "janedoe");
    }

    #[tokio::test]
    async fn test_get_current_user_returns_none_when_not_cached() {
        let mut mock_repo = MockUserRepository::new();
        mock_repo
            .expect_get_current_user()
            .times(1)
            .returning(|| Ok(None));

        let mock_tracker = MockIssueTrackerClient::new();

        let service = UserService::new(Arc::new(mock_repo), Arc::new(mock_tracker));

        let result = service.get_current_user().await.unwrap();

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_get_current_user_propagates_error() {
        let mut mock_repo = MockUserRepository::new();
        mock_repo
            .expect_get_current_user()
            .times(1)
            .returning(|| Err(WorklogError::StorageError("Connection failed".to_string())));

        let mock_tracker = MockIssueTrackerClient::new();

        let service = UserService::new(Arc::new(mock_repo), Arc::new(mock_tracker));

        let result = service.get_current_user().await;

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Connection failed"));
    }
}
