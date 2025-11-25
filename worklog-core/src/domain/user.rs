//! User domain type

use serde::{Deserialize, Serialize};

/// A user in the system
///
/// This represents the current authenticated user, typically synced
/// from the issue tracker.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct User {
    /// Unique identifier for the user (provider-specific)
    pub id: String,

    /// Display name
    pub display_name: String,

    /// Email address
    pub email: Option<String>,

    /// Timezone (e.g., `"America/New_York"`, `"Europe/Oslo"`)
    pub timezone: Option<String>,
}

impl User {
    /// Create a new user with minimal information
    pub fn new(id: impl Into<String>, display_name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            display_name: display_name.into(),
            email: None,
            timezone: None,
        }
    }

    /// Create a new user with all details
    pub fn with_details(
        id: impl Into<String>,
        display_name: impl Into<String>,
        email: Option<String>,
        timezone: Option<String>,
    ) -> Self {
        Self {
            id: id.into(),
            display_name: display_name.into(),
            email,
            timezone,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_user() {
        let user = User::new("user123", "John Doe");
        assert_eq!(user.id, "user123");
        assert_eq!(user.display_name, "John Doe");
        assert!(user.email.is_none());
        assert!(user.timezone.is_none());
    }

    #[test]
    fn test_with_details() {
        let user = User::with_details(
            "user123",
            "John Doe",
            Some("john@example.com".into()),
            Some("America/New_York".into()),
        );

        assert_eq!(user.email, Some("john@example.com".into()));
        assert_eq!(user.timezone, Some("America/New_York".into()));
    }
}
