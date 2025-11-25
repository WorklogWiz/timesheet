//! Issue key value object

use serde::{Deserialize, Serialize};
use std::fmt;

/// A validated issue key (e.g., "PROJ-123", "ABC-456")
///
/// This is a value object that ensures issue keys are well-formed.
/// It's case-insensitive and normalizes to uppercase.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct IssueKey {
    value: String,
}

impl IssueKey {
    /// Create a new issue key from a string
    ///
    /// # Arguments
    /// * `key` - The issue key string (e.g., "PROJ-123")
    ///
    /// # Panics
    /// Panics if the key is empty or whitespace-only
    pub fn new(key: impl Into<String>) -> Self {
        let value = key.into().trim().to_uppercase();
        assert!(!value.is_empty(), "Issue key cannot be empty or whitespace");
        Self { value }
    }

    /// Try to create a new issue key, returning None if invalid
    pub fn try_new(key: impl Into<String>) -> Option<Self> {
        let value = key.into().trim().to_uppercase();
        if value.is_empty() {
            None
        } else {
            Some(Self { value })
        }
    }

    /// Get the issue key as a string slice
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.value
    }

    /// Get the length of the issue key
    #[must_use]
    pub fn len(&self) -> usize {
        self.value.len()
    }

    /// Check if the issue key is empty
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.value.is_empty()
    }

    /// Extract the project prefix (e.g., "PROJ" from "PROJ-123")
    #[must_use]
    pub fn project_prefix(&self) -> Option<&str> {
        self.value.split('-').next()
    }

    /// Extract the issue number (e.g., "123" from "PROJ-123")
    #[must_use]
    pub fn issue_number(&self) -> Option<&str> {
        self.value.split('-').nth(1)
    }
}

impl fmt::Display for IssueKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.value)
    }
}

impl From<String> for IssueKey {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

impl From<&str> for IssueKey {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

impl AsRef<str> for IssueKey {
    fn as_ref(&self) -> &str {
        &self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_issue_key_creation() {
        let key = IssueKey::new("PROJ-123");
        assert_eq!(key.as_str(), "PROJ-123");
    }

    #[test]
    fn test_issue_key_uppercase() {
        let key = IssueKey::new("proj-123");
        assert_eq!(key.as_str(), "PROJ-123");
    }

    #[test]
    fn test_issue_key_trim() {
        let key = IssueKey::new("  PROJ-123  ");
        assert_eq!(key.as_str(), "PROJ-123");
    }

    #[test]
    #[should_panic(expected = "Issue key cannot be empty")]
    fn test_issue_key_empty() {
        IssueKey::new("");
    }

    #[test]
    fn test_try_new_valid() {
        let key = IssueKey::try_new("PROJ-123");
        assert!(key.is_some());
        assert_eq!(key.unwrap().as_str(), "PROJ-123");
    }

    #[test]
    fn test_try_new_invalid() {
        let key = IssueKey::try_new("");
        assert!(key.is_none());
    }

    #[test]
    fn test_project_prefix() {
        let key = IssueKey::new("PROJ-123");
        assert_eq!(key.project_prefix(), Some("PROJ"));
    }

    #[test]
    fn test_issue_number() {
        let key = IssueKey::new("PROJ-123");
        assert_eq!(key.issue_number(), Some("123"));
    }
}
