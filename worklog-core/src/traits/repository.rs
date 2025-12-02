//! Repository trait definitions for persistence

use crate::domain::{Issue, User, WorklogEntry};
use crate::error::WorklogResult;
use async_trait::async_trait;
use chrono::{DateTime, Local};

#[cfg(test)]
use mockall::automock;

/// Repository for worklog entry operations
///
/// This is a unified repository that handles both active timers and
/// completed worklogs (they're the same type, just different state).
#[cfg_attr(test, automock)]
#[async_trait]
pub trait WorklogRepository: Send + Sync {
    /// Add a new worklog entry
    ///
    /// Returns the ID of the created entry.
    async fn add(&self, entry: &WorklogEntry) -> WorklogResult<String>;

    /// Update an existing worklog entry
    async fn update(&self, entry: &WorklogEntry) -> WorklogResult<()>;

    /// Find a worklog entry by ID
    async fn find_by_id(&self, id: &str) -> WorklogResult<Option<WorklogEntry>>;

    /// Find the currently active worklog entry (timer)
    ///
    /// Returns the entry where `stopped_at IS NULL`.
    /// There should only be one active entry at a time.
    async fn find_active(&self) -> WorklogResult<Option<WorklogEntry>>;

    /// Find all worklog entries
    async fn find_all(&self) -> WorklogResult<Vec<WorklogEntry>>;

    /// Find worklog entries that started after a specific date
    async fn find_after(&self, after: DateTime<Local>) -> WorklogResult<Vec<WorklogEntry>>;

    /// Find worklog entries by issue key
    async fn find_by_issue(&self, issue_key: &str) -> WorklogResult<Vec<WorklogEntry>>;

    /// Find worklog entries that need to be synced
    ///
    /// Returns entries where:
    /// - `stopped_at IS NOT NULL` (completed)
    /// - `synced_to_provider = false`
    /// - `issue_key IS NOT NULL`
    async fn find_unsynced(&self) -> WorklogResult<Vec<WorklogEntry>>;

    /// Delete a worklog entry
    async fn delete(&self, id: &str) -> WorklogResult<()>;

    /// Count total entries
    async fn count(&self) -> WorklogResult<i64>;

    /// Check if a provider worklog ID is tombstoned (deleted locally)
    ///
    /// Returns true if the entry was deleted locally and shouldn't be re-added
    /// from remote during sync.
    async fn is_tombstoned(&self, provider_worklog_id: &str) -> WorklogResult<bool>;

    /// Clean up expired tombstones
    ///
    /// Removes tombstones older than their `expires_at` timestamp.
    /// Returns the number of tombstones removed.
    async fn cleanup_tombstones(&self) -> WorklogResult<usize>;

    /// Find entry by provider worklog ID (including soft-deleted)
    ///
    /// This is used during sync to find entries that might be soft-deleted
    /// or need updating.
    async fn find_by_provider_id(
        &self,
        provider_worklog_id: &str,
    ) -> WorklogResult<Option<WorklogEntry>>;
}

/// Repository for issue operations
#[cfg_attr(test, automock)]
#[async_trait]
pub trait IssueRepository: Send + Sync {
    /// Add or update issues
    async fn add_issues(&self, issues: &[Issue]) -> WorklogResult<()>;

    /// Find an issue by key
    async fn find_by_key(&self, key: &str) -> WorklogResult<Option<Issue>>;

    /// Find issues by keys
    async fn find_by_keys(&self, keys: &[String]) -> WorklogResult<Vec<Issue>>;

    /// Find all issues
    async fn find_all(&self) -> WorklogResult<Vec<Issue>>;

    /// Search issues by text (searches in key, summary, description, tags)
    async fn search(&self, query: &str) -> WorklogResult<Vec<Issue>>;

    /// Get unique issue keys that have worklog entries
    async fn find_keys_with_worklogs(&self) -> WorklogResult<Vec<String>>;

    /// Delete an issue
    async fn delete(&self, key: &str) -> WorklogResult<()>;
}

/// Repository for user operations
#[cfg_attr(test, automock)]
#[async_trait]
pub trait UserRepository: Send + Sync {
    /// Save or update the current user
    async fn save_current_user(&self, user: &User) -> WorklogResult<()>;

    /// Get the current user
    async fn get_current_user(&self) -> WorklogResult<Option<User>>;
}
