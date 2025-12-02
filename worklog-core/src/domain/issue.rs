//! Issue domain type

use serde::{Deserialize, Serialize};

/// A generic issue from an issue tracking system
///
/// This type works with any issue tracker (Jira, GitHub, Linear, etc.).
/// Provider-specific fields should be converted to this common format.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Issue {
    /// Unique key for the issue (e.g., "PROJ-123", "#456")
    pub key: String,

    /// Human-readable title/summary
    pub summary: String,

    /// Optional detailed description
    pub description: Option<String>,

    /// Generic tags/labels (replaces provider-specific concepts like Jira components)
    ///
    /// Examples:
    /// - Jira: `["Backend", "API"]` (from components)
    /// - GitHub: `["bug", "enhancement"]` (from labels)
    /// - Linear: `["Frontend", "Mobile"]` (from labels)
    #[serde(default)]
    pub tags: Vec<String>,

    /// Internal ID used by the provider (e.g., Jira numeric ID)
    pub provider_id: Option<String>,
}

impl Issue {
    /// Create a new issue with minimal information
    pub fn new(key: impl Into<String>, summary: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            summary: summary.into(),
            description: None,
            tags: Vec::new(),
            provider_id: None,
        }
    }

    /// Create a new issue with all fields
    pub fn with_details(
        key: impl Into<String>,
        summary: impl Into<String>,
        description: Option<String>,
        tags: Vec<String>,
        provider_id: Option<String>,
    ) -> Self {
        Self {
            key: key.into(),
            summary: summary.into(),
            description,
            tags,
            provider_id,
        }
    }

    /// Check if this issue has a specific tag
    #[must_use]
    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags.iter().any(|t| t.eq_ignore_ascii_case(tag))
    }

    /// Add a tag if not already present
    pub fn add_tag(&mut self, tag: String) {
        if !self.has_tag(&tag) {
            self.tags.push(tag);
        }
    }

    /// Remove a tag
    pub fn remove_tag(&mut self, tag: &str) {
        self.tags.retain(|t| !t.eq_ignore_ascii_case(tag));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_issue() {
        let issue = Issue::new("PROJ-123", "Fix bug");
        assert_eq!(issue.key, "PROJ-123");
        assert_eq!(issue.summary, "Fix bug");
        assert!(issue.description.is_none());
        assert!(issue.tags.is_empty());
    }

    #[test]
    fn test_with_details() {
        let issue = Issue::with_details(
            "PROJ-123",
            "Fix bug",
            Some("Detailed description".into()),
            vec!["backend".into(), "urgent".into()],
            Some("12345".into()),
        );

        assert_eq!(issue.key, "PROJ-123");
        assert_eq!(issue.description, Some("Detailed description".into()));
        assert_eq!(issue.tags, vec!["backend", "urgent"]);
        assert_eq!(issue.provider_id, Some("12345".into()));
    }

    #[test]
    fn test_tags() {
        let mut issue = Issue::new("PROJ-1", "Test");

        issue.add_tag("backend".into());
        assert!(issue.has_tag("backend"));
        assert!(issue.has_tag("BACKEND")); // Case-insensitive

        issue.add_tag("backend".into()); // Duplicate
        assert_eq!(issue.tags.len(), 1); // Not added

        issue.remove_tag("backend");
        assert!(!issue.has_tag("backend"));
    }
}
