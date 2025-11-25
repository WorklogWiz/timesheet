//! Unified worklog entry (combines timer and worklog concepts)

use crate::domain::IssueKey;
use chrono::{DateTime, Duration, Local};
use serde::{Deserialize, Serialize};

/// A worklog entry representing tracked time
///
/// This type unifies the concepts of "timer" and "worklog":
/// - `stopped_at = None` → Active timer (work in progress)
/// - `stopped_at = Some(time)` → Completed worklog (work finished)
///
/// # Examples
///
/// ## Starting a timer
///
/// ```rust
/// use worklog_core::domain::WorklogEntry;
/// use chrono::Local;
///
/// let entry = WorklogEntry::start_now(
///     Some("PROJ-123".to_string()),
///     Some("Working on feature".to_string())
/// );
///
/// assert!(entry.is_active());
/// assert_eq!(entry.issue_key.as_deref(), Some("PROJ-123"));
/// ```
///
/// ## Stopping a timer
///
/// ```rust
/// use worklog_core::domain::WorklogEntry;
/// use chrono::Local;
///
/// let mut entry = WorklogEntry::start_now(None, None);
/// entry.stop(Local::now());
///
/// assert!(!entry.is_active());
/// assert!(entry.duration().is_some());
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorklogEntry {
    /// Unique identifier for this entry
    pub id: Option<String>,

    /// Optional issue key (can track work without an issue)
    pub issue_key: Option<String>,

    /// When the work started
    pub started_at: DateTime<Local>,

    /// When the work stopped (None = still active/timer running)
    pub stopped_at: Option<DateTime<Local>>,

    /// Optional comment describing the work
    pub comment: Option<String>,

    /// Generic tags (e.g., `["backend", "api", "bug-fix"]`)
    #[serde(default)]
    pub tags: Vec<String>,

    /// Time spent in seconds (calculated from `stopped_at` - `started_at`)
    pub time_spent_seconds: Option<i32>,

    /// Has this been synced to an external provider (Jira, etc.)?
    #[serde(default)]
    pub synced_to_provider: bool,

    /// ID from external provider (e.g., Jira worklog ID)
    pub provider_worklog_id: Option<String>,

    /// When this record was created
    #[serde(default = "chrono::Local::now")]
    pub created_at: DateTime<Local>,

    /// When this record was last updated
    #[serde(default = "chrono::Local::now")]
    pub updated_at: DateTime<Local>,

    /// When this record was soft-deleted (None = not deleted)
    ///
    /// Used for Git-like sync: deleted items are marked, not removed,
    /// to prevent resurrection when syncing from remote
    #[serde(default)]
    pub deleted_at: Option<DateTime<Local>>,

    /// When this record was last synced with remote tracker
    ///
    /// Used for conflict detection: if local `updated_at` > `last_synced_at`,
    /// the item was modified locally since last sync
    #[serde(default)]
    pub last_synced_at: Option<DateTime<Local>>,

    /// Version number for optimistic locking (increments on each update)
    ///
    /// Used for conflict detection: if local version != remote version,
    /// the item was modified in both places (conflict)
    #[serde(default = "default_version")]
    pub version: i32,

    /// Whether this entry has a sync conflict (local and remote both modified)
    ///
    /// When true, user intervention is required to resolve the conflict
    #[serde(default)]
    pub has_conflict: bool,
}

fn default_version() -> i32 {
    1
}

impl WorklogEntry {
    /// Start a new worklog entry (timer) with the current time
    ///
    /// # Arguments
    /// * `issue_key` - Optional issue key to associate with this work
    /// * `comment` - Optional description of the work
    ///
    /// # Returns
    /// A new active worklog entry (timer)
    #[must_use]
    pub fn start_now(issue_key: Option<String>, comment: Option<String>) -> Self {
        let now = Local::now();
        Self {
            id: None,
            issue_key,
            started_at: now,
            stopped_at: None, // Active timer!
            comment,
            tags: Vec::new(),
            time_spent_seconds: None,
            synced_to_provider: false,
            provider_worklog_id: None,
            created_at: now,
            updated_at: now,
            deleted_at: None,
            last_synced_at: None,
            version: 1,
            has_conflict: false,
        }
    }

    /// Start a new worklog entry at a specific time
    #[must_use]
    pub fn start_at(
        started_at: DateTime<Local>,
        issue_key: Option<String>,
        comment: Option<String>,
    ) -> Self {
        let now = Local::now();
        Self {
            id: None,
            issue_key,
            started_at,
            stopped_at: None,
            comment,
            tags: Vec::new(),
            time_spent_seconds: None,
            synced_to_provider: false,
            provider_worklog_id: None,
            created_at: now,
            updated_at: now,
            deleted_at: None,
            last_synced_at: None,
            version: 1,
            has_conflict: false,
        }
    }

    /// Check if this entry is active (timer is running)
    ///
    /// An entry is active when `stopped_at` is `None`.
    #[must_use]
    pub fn is_active(&self) -> bool {
        self.stopped_at.is_none()
    }

    /// Check if this entry can be synced to a provider
    ///
    /// An entry can be synced if:
    /// - It has an `issue_key` (providers require this)
    /// - It is not active (has been stopped)
    /// - It has not already been synced
    #[must_use]
    pub fn can_sync(&self) -> bool {
        self.issue_key.is_some() && !self.is_active() && !self.synced_to_provider
    }

    /// Calculate the duration of this entry
    ///
    /// Returns None if the entry is still active (no `stopped_at` time).
    #[must_use]
    pub fn duration(&self) -> Option<Duration> {
        self.stopped_at.map(|stop| stop - self.started_at)
    }

    /// Get the duration in seconds
    #[must_use]
    pub fn duration_seconds(&self) -> Option<i32> {
        self.duration()
            .and_then(|d| i32::try_from(d.num_seconds()).ok())
    }

    /// Stop this worklog entry (convert from timer to completed worklog)
    ///
    /// # Arguments
    /// * `at` - The time when work stopped
    ///
    /// This method:
    /// - Sets `stopped_at` to the provided time
    /// - Calculates `time_spent_seconds` from the duration
    /// - Updates `updated_at` to now
    /// - Increments version
    ///
    /// If the entry is already stopped, this does nothing.
    pub fn stop(&mut self, at: DateTime<Local>) {
        if self.is_active() {
            self.stopped_at = Some(at);
            self.time_spent_seconds = i32::try_from((at - self.started_at).num_seconds()).ok();
            self.updated_at = Local::now();
            self.version += 1;
        }
    }

    /// Stop this worklog entry at the current time
    pub fn stop_now(&mut self) {
        self.stop(Local::now());
    }

    /// Update the comment
    pub fn set_comment(&mut self, comment: Option<String>) {
        self.comment = comment;
        self.updated_at = Local::now();
        self.version += 1;
    }

    /// Update the issue key
    pub fn set_issue_key(&mut self, issue_key: Option<String>) {
        self.issue_key = issue_key;
        self.updated_at = Local::now();
        self.version += 1;
    }

    /// Add a tag
    pub fn add_tag(&mut self, tag: String) {
        if !self.tags.contains(&tag) {
            self.tags.push(tag);
            self.updated_at = Local::now();
            self.version += 1;
        }
    }

    /// Remove a tag
    pub fn remove_tag(&mut self, tag: &str) {
        if let Some(pos) = self.tags.iter().position(|t| t == tag) {
            self.tags.remove(pos);
            self.updated_at = Local::now();
            self.version += 1;
        }
    }

    /// Mark as synced to provider
    pub fn mark_synced(&mut self, provider_worklog_id: Option<String>) {
        let now = Local::now();
        self.synced_to_provider = true;
        self.provider_worklog_id = provider_worklog_id;
        self.last_synced_at = Some(now);
        self.updated_at = now;
    }

    /// Soft delete this entry (mark as deleted without removing from DB)
    ///
    /// Used for Git-like sync to prevent resurrection when syncing from remote
    pub fn soft_delete(&mut self) {
        let now = Local::now();
        self.deleted_at = Some(now);
        self.updated_at = now;
        self.version += 1;
    }

    /// Check if this entry is soft-deleted
    #[must_use]
    pub fn is_deleted(&self) -> bool {
        self.deleted_at.is_some()
    }

    /// Increment the version number (for optimistic locking)
    pub fn increment_version(&mut self) {
        self.version += 1;
    }

    /// Check if this entry was modified locally since last sync
    ///
    /// Returns true if:
    /// - `updated_at` > `last_synced_at` (local changes exist)
    /// - Never been synced (`last_synced_at` is None)
    #[must_use]
    pub fn has_local_changes(&self) -> bool {
        match self.last_synced_at {
            Some(last_sync) => self.updated_at > last_sync,
            None => true, // Never synced = has changes
        }
    }

    /// Get the issue key as an `IssueKey` value object
    #[must_use]
    pub fn issue_key_obj(&self) -> Option<IssueKey> {
        self.issue_key.as_ref().map(IssueKey::new)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn test_start_now() {
        let entry = WorklogEntry::start_now(Some("TEST-1".into()), Some("Working".into()));

        assert!(entry.is_active());
        assert_eq!(entry.issue_key, Some("TEST-1".into()));
        assert_eq!(entry.comment, Some("Working".into()));
        assert!(!entry.synced_to_provider);
        assert!(entry.stopped_at.is_none());
        assert!(entry.time_spent_seconds.is_none());
    }

    #[test]
    fn test_stop() {
        let mut entry = WorklogEntry::start_now(None, None);
        let start = entry.started_at;

        // Simulate working for 1 hour
        let stop_time = start + Duration::hours(1);
        entry.stop(stop_time);

        assert!(!entry.is_active());
        assert_eq!(entry.stopped_at, Some(stop_time));
        assert_eq!(entry.time_spent_seconds, Some(3600));
        assert_eq!(entry.duration_seconds(), Some(3600));
    }

    #[test]
    fn test_stop_already_stopped() {
        let mut entry = WorklogEntry::start_now(None, None);
        let start = entry.started_at;

        entry.stop(start + Duration::hours(1));
        let first_stop = entry.stopped_at;

        // Try to stop again - should not change
        entry.stop(start + Duration::hours(2));
        assert_eq!(entry.stopped_at, first_stop);
    }

    #[test]
    fn test_can_sync() {
        let mut entry = WorklogEntry::start_now(Some("TEST-1".into()), None);

        // Active entry cannot sync
        assert!(!entry.can_sync());

        // Stop it
        entry.stop_now();
        assert!(entry.can_sync());

        // After sync, cannot sync again
        entry.mark_synced(Some("jira-123".into()));
        assert!(!entry.can_sync());
    }

    #[test]
    fn test_can_sync_without_issue() {
        let mut entry = WorklogEntry::start_now(None, None);
        entry.stop_now();

        // Cannot sync without issue key
        assert!(!entry.can_sync());
    }

    #[test]
    fn test_tags() {
        let mut entry = WorklogEntry::start_now(None, None);

        entry.add_tag("backend".into());
        entry.add_tag("api".into());
        assert_eq!(entry.tags, vec!["backend", "api"]);

        // Adding duplicate does nothing
        entry.add_tag("backend".into());
        assert_eq!(entry.tags, vec!["backend", "api"]);

        entry.remove_tag("backend");
        assert_eq!(entry.tags, vec!["api"]);
    }

    #[test]
    fn test_set_comment() {
        let mut entry = WorklogEntry::start_now(None, None);
        entry.set_comment(Some("New comment".into()));
        assert_eq!(entry.comment, Some("New comment".into()));
    }

    #[test]
    fn test_set_issue_key() {
        let mut entry = WorklogEntry::start_now(None, None);
        entry.set_issue_key(Some("TEST-123".into()));
        assert_eq!(entry.issue_key, Some("TEST-123".into()));
    }
}
