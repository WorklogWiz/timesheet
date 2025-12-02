//! Unified worklog service module - manages both active timers and completed worklogs.
//!
//! This module provides the `WorklogService`, which handles:
//! - Starting and stopping work timers (active worklogs)
//! - Managing completed worklog entries
//! - Synchronizing worklogs with external issue trackers
//! - Calculating time spent on issues
//!
//! This service operates on `WorklogEntry`, which unifies the concepts
//! of "timer" (active worklog with `stopped_at = None`) and "worklog" (completed entry).

use crate::domain::WorklogEntry;
use crate::error::WorklogError;
use crate::traits::{IssueTrackerClient, WorklogRepository};
use chrono::{DateTime, Duration, Local, Utc};
use log::{debug, info};
use std::sync::Arc;

/// Unified service for managing worklog entries (both active timers and completed worklogs)
pub struct WorklogService {
    repository: Arc<dyn WorklogRepository>,
    issue_tracker: Arc<dyn IssueTrackerClient>,
}

#[allow(clippy::missing_errors_doc)]
impl WorklogService {
    /// Creates a new `WorklogService` instance
    ///
    /// # Arguments
    ///
    /// * `repository` - Repository for persisting worklog entries
    /// * `issue_tracker` - Client for syncing to external issue trackers
    pub fn new(
        repository: Arc<dyn WorklogRepository>,
        issue_tracker: Arc<dyn IssueTrackerClient>,
    ) -> Self {
        Self {
            repository,
            issue_tracker,
        }
    }

    /// Starts a new timer (active worklog) for the specified issue
    ///
    /// Creates an entry in the local database with `stopped_at = None`.
    /// The `issue_key` is optional - you can track local work without an issue.
    ///
    /// # Arguments
    ///
    /// * `issue_key` - Optional issue key to associate with this worklog
    /// * `started_at` - When the timer started
    /// * `comment` - Optional comment describing the work
    ///
    /// # Returns
    ///
    /// The created `WorklogEntry` with `is_active() == true`
    ///
    /// # Errors
    ///
    /// Returns `WorklogError` if:
    /// - There is already an active timer running
    /// - Repository operations fail
    pub async fn start_timer(
        &self,
        issue_key: Option<String>,
        started_at: DateTime<Local>,
        comment: Option<String>,
    ) -> Result<WorklogEntry, WorklogError> {
        debug!(
            "Starting timer for issue: {:?}",
            issue_key.as_deref().unwrap_or("(local)")
        );

        // Check if there's already an active timer
        if self.repository.find_active().await?.is_some() {
            return Err(WorklogError::ActiveEntryExists);
        }

        // Create a new active worklog entry
        let mut entry = WorklogEntry::start_now(issue_key, comment);
        entry.started_at = started_at;

        let id = self.repository.add(&entry).await?;
        entry.id = Some(id.clone());

        debug!("Started timer with id: {id}");
        Ok(entry)
    }

    /// Stops the currently active timer and syncs to issue tracker
    ///
    /// Sets `stopped_at` to the provided time, calculates `time_spent_seconds`,
    /// and automatically syncs to the configured issue tracker if the entry has an issue key.
    ///
    /// # Arguments
    ///
    /// * `stopped_at` - When the timer stopped
    /// * `comment` - Optional comment to set (overrides existing comment if provided)
    ///
    /// # Returns
    ///
    /// The stopped `WorklogEntry` with `is_active() == false`
    ///
    /// # Errors
    ///
    /// Returns `WorklogError` if:
    /// - No active timer exists
    /// - Timer duration is too small (< 60 seconds)
    /// - Repository operations fail
    ///
    /// Note: If syncing to the issue tracker fails, a warning is logged but the operation
    /// succeeds. The entry will be marked as unsynced and can be synced later.
    pub async fn stop_active_timer(
        &self,
        stopped_at: DateTime<Local>,
        comment: Option<String>,
    ) -> Result<WorklogEntry, WorklogError> {
        const MIN_DURATION_SECONDS: i64 = 60;

        debug!("Stopping active timer");

        let mut entry = self
            .repository
            .find_active()
            .await?
            .ok_or(WorklogError::NoActiveEntry)?;

        // Calculate duration
        let duration = stopped_at - entry.started_at;

        if duration.num_seconds() < MIN_DURATION_SECONDS {
            #[allow(clippy::cast_possible_truncation)]
            let duration_seconds = duration.num_seconds() as i32;
            return Err(WorklogError::ValidationError(format!(
                "Timer duration too small: {duration_seconds}s. Must be at least 1 minute."
            )));
        }

        // Stop the timer
        entry.stop(stopped_at);

        // Update comment if provided
        if let Some(new_comment) = comment {
            entry.set_comment(Some(new_comment));
        }

        // Save the stopped entry to local DB
        self.repository.update(&entry).await?;

        info!(
            "Stopped timer {} - Duration: {:?}",
            entry.id.as_ref().unwrap_or(&"unknown".to_string()),
            entry.duration()
        );

        // Try to sync to issue tracker if entry can be synced
        if entry.can_sync() {
            match self.issue_tracker.sync_worklog(&entry).await {
                Ok(provider_id) => {
                    debug!(
                        "Synced stopped timer {} to issue tracker with ID: {}",
                        entry.id.as_deref().unwrap_or("unknown"),
                        provider_id
                    );

                    // Update entry with provider ID and sync status
                    entry.provider_worklog_id = Some(provider_id);
                    entry.synced_to_provider = true;

                    // Save the updated sync status
                    if let Err(e) = self.repository.update(&entry).await {
                        log::error!("Failed to update sync status for worklog: {e}");
                    } else {
                        info!("Worklog synced to issue tracker successfully");
                    }
                }
                Err(e) => {
                    log::warn!(
                        "Failed to sync worklog to issue tracker: {e}. Entry saved locally and can be synced later."
                    );
                    // Don't fail the operation - entry is saved locally
                }
            }
        } else {
            debug!("Entry cannot be synced (no issue key or still active)");
        }

        Ok(entry)
    }

    /// Gets the currently active timer, if any
    ///
    /// # Returns
    ///
    /// `Some(WorklogEntry)` if there's an active timer, `None` otherwise
    pub async fn get_active_timer(&self) -> Result<Option<WorklogEntry>, WorklogError> {
        self.repository.find_active().await
    }

    /// Discards (deletes) the currently active timer without saving it
    ///
    /// # Returns
    ///
    /// The deleted `WorklogEntry`
    ///
    /// # Errors
    ///
    /// Returns `WorklogError` if no active timer exists
    pub async fn discard_active_timer(&self) -> Result<WorklogEntry, WorklogError> {
        let entry = self
            .repository
            .find_active()
            .await?
            .ok_or(WorklogError::NoActiveEntry)?;

        let id = entry
            .id
            .as_ref()
            .ok_or_else(|| WorklogError::ValidationError("Entry has no ID".to_string()))?;

        self.repository.delete(id).await?;

        debug!("Discarded active timer: {id}");
        Ok(entry)
    }

    /// Updates the comment on the active timer or a specific worklog entry
    ///
    /// # Arguments
    ///
    /// * `id` - ID of the worklog entry to update
    /// * `comment` - New comment (None to clear)
    pub async fn update_comment(
        &self,
        id: &str,
        comment: Option<String>,
    ) -> Result<(), WorklogError> {
        let mut entry =
            self.repository.find_by_id(id).await?.ok_or_else(|| {
                WorklogError::EntryNotFound(format!("Worklog entry {id} not found"))
            })?;

        entry.set_comment(comment);
        self.repository.update(&entry).await?;

        debug!("Updated comment for worklog: {id}");
        Ok(())
    }

    /// Updates the issue key for a worklog entry
    ///
    /// # Arguments
    ///
    /// * `id` - ID of the worklog entry to update
    /// * `issue_key` - New issue key (None to clear)
    pub async fn update_issue_key(
        &self,
        id: &str,
        issue_key: Option<String>,
    ) -> Result<(), WorklogError> {
        let mut entry =
            self.repository.find_by_id(id).await?.ok_or_else(|| {
                WorklogError::EntryNotFound(format!("Worklog entry {id} not found"))
            })?;

        entry.set_issue_key(issue_key);
        self.repository.update(&entry).await?;

        debug!("Updated issue key for worklog: {id}");
        Ok(())
    }

    /// Adds or removes a tag from a worklog entry
    ///
    /// # Arguments
    ///
    /// * `id` - ID of the worklog entry
    /// * `tag` - Tag to add or remove
    /// * `add` - If true, adds the tag; if false, removes it
    pub async fn update_tag(&self, id: &str, tag: &str, add: bool) -> Result<(), WorklogError> {
        let mut entry =
            self.repository.find_by_id(id).await?.ok_or_else(|| {
                WorklogError::EntryNotFound(format!("Worklog entry {id} not found"))
            })?;

        if add {
            entry.add_tag(tag.to_string());
        } else {
            entry.remove_tag(tag);
        }

        self.repository.update(&entry).await?;
        debug!("Updated tag '{tag}' for worklog: {id}");
        Ok(())
    }

    /// Finds all worklog entries (both active and completed)
    pub async fn find_all(&self) -> Result<Vec<WorklogEntry>, WorklogError> {
        self.repository.find_all().await
    }

    /// Finds worklog entries after a specific date
    ///
    /// # Arguments
    ///
    /// * `after` - Only return entries that started after this date
    pub async fn find_after(
        &self,
        after: DateTime<Utc>,
    ) -> Result<Vec<WorklogEntry>, WorklogError> {
        // Convert to Local for comparison
        let after_local = after.with_timezone(&Local);

        self.repository.find_after(after_local).await
    }

    /// Finds worklog entries for a specific issue
    ///
    /// # Arguments
    ///
    /// * `issue_key` - Issue key to filter by
    pub async fn find_by_issue(&self, issue_key: &str) -> Result<Vec<WorklogEntry>, WorklogError> {
        self.repository.find_by_issue(issue_key).await
    }

    /// Calculates total time spent on a specific issue
    ///
    /// Sums the duration of all completed worklogs for the given issue.
    ///
    /// # Arguments
    ///
    /// * `issue_key` - Issue key to calculate time for
    ///
    /// # Returns
    ///
    /// Total duration as `chrono::Duration`
    pub async fn get_total_time_for_issue(
        &self,
        issue_key: &str,
    ) -> Result<Duration, WorklogError> {
        let worklogs = self.find_by_issue(issue_key).await?;

        let total_seconds: i64 = worklogs
            .iter()
            .filter_map(|entry| entry.time_spent_seconds)
            .map(i64::from)
            .sum();

        Ok(Duration::seconds(total_seconds))
    }

    /// Deletes a worklog entry by ID
    ///
    /// # Arguments
    ///
    /// * `id` - ID of the worklog entry to delete
    pub async fn delete(&self, id: &str) -> Result<(), WorklogError> {
        self.repository.delete(id).await?;
        debug!("Deleted worklog: {id}");
        Ok(())
    }

    /// Find a worklog entry by ID
    ///
    /// # Arguments
    ///
    /// * `id` - The unique ID of the worklog entry
    ///
    /// # Returns
    ///
    /// The `WorklogEntry` if found, None otherwise
    ///
    /// # Errors
    ///
    /// Returns `WorklogError` if repository operations fail
    pub async fn find_by_id(&self, id: &str) -> Result<Option<WorklogEntry>, WorklogError> {
        self.repository.find_by_id(id).await
    }

    /// Update an existing worklog entry
    ///
    /// # Arguments
    ///
    /// * `entry` - The worklog entry to update
    ///
    /// # Errors
    ///
    /// Returns `WorklogError` if repository operations fail
    pub async fn update_entry(&self, entry: &WorklogEntry) -> Result<(), WorklogError> {
        self.repository.update(entry).await?;
        debug!("Updated worklog: {:?}", entry.id);
        Ok(())
    }

    /// Add a new worklog entry (for direct addition, not timer-based)
    ///
    /// # Arguments
    ///
    /// * `entry` - The worklog entry to add
    ///
    /// # Returns
    ///
    /// The ID of the newly added entry
    ///
    /// # Errors
    ///
    /// Returns `WorklogError` if repository operations fail
    pub async fn add_entry(&self, entry: &WorklogEntry) -> Result<String, WorklogError> {
        let id = self.repository.add(entry).await?;
        debug!("Added worklog entry with id: {id}");
        Ok(id)
    }

    /// Add a worklog entry and immediately sync it to the issue tracker
    ///
    /// This is useful for CLI operations where you want immediate feedback.
    /// For batch operations or UI, consider using `add_entry()` + `SyncService`.
    ///
    /// # Arguments
    ///
    /// * `entry` - The worklog entry to add
    ///
    /// # Returns
    ///
    /// The complete worklog entry with local ID and provider ID
    ///
    /// # Errors
    ///
    /// Returns `WorklogError` if local add or sync to provider fails
    pub async fn add_and_sync(&self, entry: &WorklogEntry) -> Result<WorklogEntry, WorklogError> {
        // First sync to issue tracker
        let provider_id = self.issue_tracker.sync_worklog(entry).await?;

        // Then save locally with provider ID
        let mut saved_entry = entry.clone();
        saved_entry.provider_worklog_id = Some(provider_id.clone());
        saved_entry.synced_to_provider = true;
        saved_entry.last_synced_at = Some(chrono::Local::now());

        let local_id = self.repository.add(&saved_entry).await?;
        saved_entry.id = Some(local_id);

        info!(
            "Added and synced worklog: local_id={}, provider_id={}",
            saved_entry.id.as_deref().unwrap_or("unknown"),
            provider_id
        );

        Ok(saved_entry)
    }

    /// Delete a worklog entry both locally and from the issue tracker
    ///
    /// This is useful for CLI operations where you want immediate deletion.
    /// For UI operations, consider using `delete()` + `SyncService` for proper tombstone handling.
    ///
    /// # Arguments
    ///
    /// * `id` - The local ID of the worklog entry to delete
    ///
    /// # Errors
    ///
    /// Returns `WorklogError` if the entry is not found, or if local/remote deletion fails
    pub async fn delete_and_sync(&self, id: &str) -> Result<(), WorklogError> {
        // First, get the entry to find the provider worklog ID
        let entry =
            self.repository.find_by_id(id).await?.ok_or_else(|| {
                WorklogError::EntryNotFound(format!("Worklog entry not found: {id}"))
            })?;

        // If it was synced to provider, delete from there first
        if entry.synced_to_provider {
            if let (Some(issue_key), Some(provider_id)) =
                (&entry.issue_key, &entry.provider_worklog_id)
            {
                self.issue_tracker
                    .delete_worklog(issue_key, provider_id)
                    .await?;
                info!("Deleted worklog from issue tracker: {provider_id}");
            }
        }

        // Then delete locally (this will soft-delete and create tombstone)
        self.repository.delete(id).await?;
        info!("Deleted worklog locally: {id}");

        Ok(())
    }

    /// Fetch worklogs for an issue from the remote issue tracker
    ///
    /// This fetches all worklogs for a given issue from the remote tracker,
    /// filtering by start date.
    ///
    /// # Arguments
    ///
    /// * `issue_key` - The issue key to fetch worklogs for
    /// * `since` - Fetch worklogs updated/created after this date
    ///
    /// # Returns
    ///
    /// Vector of worklog entries from the remote tracker
    ///
    /// # Errors
    ///
    /// Returns `WorklogError` if the remote fetch fails
    pub async fn get_worklogs_for_issue(
        &self,
        issue_key: &str,
        since: chrono::DateTime<chrono::Utc>,
    ) -> Result<Vec<crate::WorklogEntry>, WorklogError> {
        self.issue_tracker
            .get_worklogs_for_issue(issue_key, since)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::traits::{MockIssueTrackerClient, MockWorklogRepository};
    use mockall::predicate::*;

    // Helper to create a test worklog entry
    fn create_test_entry(
        id: Option<&str>,
        issue_key: Option<&str>,
        started_at: DateTime<Local>,
        stopped_at: Option<DateTime<Local>>,
    ) -> WorklogEntry {
        WorklogEntry {
            id: id.map(std::string::ToString::to_string),
            issue_key: issue_key.map(std::string::ToString::to_string),
            started_at,
            stopped_at,
            comment: Some("Test comment".to_string()),
            tags: vec![],
            time_spent_seconds: stopped_at.map(|st| {
                (st - started_at)
                    .num_seconds()
                    .try_into()
                    .expect("Test duration too large for i32")
            }),
            synced_to_provider: false,
            provider_worklog_id: None,
            created_at: started_at,
            updated_at: started_at,
            deleted_at: None,
            last_synced_at: None,
            version: 1,
            has_conflict: false,
        }
    }

    #[tokio::test]
    async fn test_start_timer_success() {
        let mut mock_repo = MockWorklogRepository::new();
        mock_repo
            .expect_find_active()
            .times(1)
            .returning(|| Ok(None));
        mock_repo
            .expect_add()
            .times(1)
            .returning(|_| Ok("new-id".to_string()));

        let mock_tracker = MockIssueTrackerClient::new();
        let service = WorklogService::new(Arc::new(mock_repo), Arc::new(mock_tracker));

        let now = Local::now();
        let result = service
            .start_timer(Some("PROJ-1".to_string()), now, Some("Working".to_string()))
            .await
            .unwrap();

        assert_eq!(result.id, Some("new-id".to_string()));
        assert_eq!(result.issue_key, Some("PROJ-1".to_string()));
        assert!(result.is_active());
    }

    #[tokio::test]
    async fn test_start_timer_already_active() {
        let existing_entry =
            create_test_entry(Some("existing"), Some("PROJ-1"), Local::now(), None);

        let mut mock_repo = MockWorklogRepository::new();
        mock_repo
            .expect_find_active()
            .times(1)
            .returning(move || Ok(Some(existing_entry.clone())));
        mock_repo.expect_add().times(0);

        let mock_tracker = MockIssueTrackerClient::new();
        let service = WorklogService::new(Arc::new(mock_repo), Arc::new(mock_tracker));

        let result = service
            .start_timer(Some("PROJ-2".to_string()), Local::now(), None)
            .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            WorklogError::ActiveEntryExists => {}
            _ => panic!("Expected ActiveEntryExists error"),
        }
    }

    #[tokio::test]
    async fn test_start_timer_without_issue_key() {
        let mut mock_repo = MockWorklogRepository::new();
        mock_repo
            .expect_find_active()
            .times(1)
            .returning(|| Ok(None));
        mock_repo
            .expect_add()
            .times(1)
            .returning(|_| Ok("local-id".to_string()));

        let mock_tracker = MockIssueTrackerClient::new();
        let service = WorklogService::new(Arc::new(mock_repo), Arc::new(mock_tracker));

        let result = service
            .start_timer(None, Local::now(), Some("Local work".to_string()))
            .await
            .unwrap();

        assert_eq!(result.id, Some("local-id".to_string()));
        assert_eq!(result.issue_key, None);
    }

    #[tokio::test]
    async fn test_stop_active_timer_success() {
        let start_time = Local::now() - Duration::minutes(65);
        let stop_time = Local::now();
        let active_entry = create_test_entry(Some("active-id"), Some("PROJ-1"), start_time, None);

        let active_clone = active_entry.clone();

        let mut mock_repo = MockWorklogRepository::new();
        mock_repo
            .expect_find_active()
            .times(1)
            .returning(move || Ok(Some(active_clone.clone())));
        mock_repo
            .expect_update()
            .times(2) // Once for stopping, once for sync status
            .returning(|_| Ok(()));

        let mut mock_tracker = MockIssueTrackerClient::new();
        mock_tracker
            .expect_sync_worklog()
            .times(1)
            .returning(|_| Ok("provider-123".to_string()));

        let service = WorklogService::new(Arc::new(mock_repo), Arc::new(mock_tracker));

        let result = service
            .stop_active_timer(stop_time, Some("Completed".to_string()))
            .await
            .unwrap();

        assert_eq!(result.stopped_at, Some(stop_time));
        assert_eq!(result.comment, Some("Completed".to_string()));
        assert!(!result.is_active());
    }

    #[tokio::test]
    async fn test_stop_active_timer_no_active_timer() {
        let mut mock_repo = MockWorklogRepository::new();
        mock_repo
            .expect_find_active()
            .times(1)
            .returning(|| Ok(None));

        let mock_tracker = MockIssueTrackerClient::new();
        let service = WorklogService::new(Arc::new(mock_repo), Arc::new(mock_tracker));

        let result = service.stop_active_timer(Local::now(), None).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            WorklogError::NoActiveEntry => {}
            _ => panic!("Expected NoActiveEntry error"),
        }
    }

    #[tokio::test]
    async fn test_stop_active_timer_duration_too_short() {
        let start_time = Local::now() - Duration::seconds(30); // Only 30 seconds
        let active_entry = create_test_entry(Some("active-id"), Some("PROJ-1"), start_time, None);

        let mut mock_repo = MockWorklogRepository::new();
        mock_repo
            .expect_find_active()
            .times(1)
            .returning(move || Ok(Some(active_entry.clone())));

        let mock_tracker = MockIssueTrackerClient::new();
        let service = WorklogService::new(Arc::new(mock_repo), Arc::new(mock_tracker));

        let result = service.stop_active_timer(Local::now(), None).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("too small"));
    }

    #[tokio::test]
    async fn test_stop_active_timer_sync_failure_still_saves() {
        let start_time = Local::now() - Duration::minutes(65);
        let stop_time = Local::now();
        let active_entry = create_test_entry(Some("active-id"), Some("PROJ-1"), start_time, None);
        let active_clone = active_entry.clone();

        let mut mock_repo = MockWorklogRepository::new();
        mock_repo
            .expect_find_active()
            .times(1)
            .returning(move || Ok(Some(active_clone.clone())));
        mock_repo
            .expect_update()
            .times(1) // Only once for stopping (no sync update)
            .returning(|_| Ok(()));

        let mut mock_tracker = MockIssueTrackerClient::new();
        mock_tracker
            .expect_sync_worklog()
            .times(1)
            .returning(|_| Err(WorklogError::IssueTrackerError("Network error".to_string())));

        let service = WorklogService::new(Arc::new(mock_repo), Arc::new(mock_tracker));

        // Should succeed even though sync failed (sync failure is logged, not propagated)
        let result = service.stop_active_timer(stop_time, None).await.unwrap();

        assert!(!result.is_active());
        assert_eq!(result.provider_worklog_id, None); // Not synced
    }

    #[tokio::test]
    async fn test_get_active_timer_found() {
        let active_entry = create_test_entry(Some("active-id"), Some("PROJ-1"), Local::now(), None);
        let active_clone = active_entry.clone();

        let mut mock_repo = MockWorklogRepository::new();
        mock_repo
            .expect_find_active()
            .times(1)
            .returning(move || Ok(Some(active_clone.clone())));

        let mock_tracker = MockIssueTrackerClient::new();
        let service = WorklogService::new(Arc::new(mock_repo), Arc::new(mock_tracker));

        let result = service.get_active_timer().await.unwrap();

        assert!(result.is_some());
        assert_eq!(result.unwrap().id, Some("active-id".to_string()));
    }

    #[tokio::test]
    async fn test_get_active_timer_none() {
        let mut mock_repo = MockWorklogRepository::new();
        mock_repo
            .expect_find_active()
            .times(1)
            .returning(|| Ok(None));

        let mock_tracker = MockIssueTrackerClient::new();
        let service = WorklogService::new(Arc::new(mock_repo), Arc::new(mock_tracker));

        let result = service.get_active_timer().await.unwrap();

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_discard_active_timer_success() {
        let active_entry = create_test_entry(Some("active-id"), Some("PROJ-1"), Local::now(), None);
        let active_clone = active_entry.clone();

        let mut mock_repo = MockWorklogRepository::new();
        mock_repo
            .expect_find_active()
            .times(1)
            .returning(move || Ok(Some(active_clone.clone())));
        mock_repo
            .expect_delete()
            .with(eq("active-id"))
            .times(1)
            .returning(|_| Ok(()));

        let mock_tracker = MockIssueTrackerClient::new();
        let service = WorklogService::new(Arc::new(mock_repo), Arc::new(mock_tracker));

        let result = service.discard_active_timer().await.unwrap();

        assert_eq!(result.id, Some("active-id".to_string()));
    }

    #[tokio::test]
    async fn test_discard_active_timer_no_active() {
        let mut mock_repo = MockWorklogRepository::new();
        mock_repo
            .expect_find_active()
            .times(1)
            .returning(|| Ok(None));

        let mock_tracker = MockIssueTrackerClient::new();
        let service = WorklogService::new(Arc::new(mock_repo), Arc::new(mock_tracker));

        let result = service.discard_active_timer().await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_update_comment_success() {
        let entry = create_test_entry(
            Some("id-1"),
            Some("PROJ-1"),
            Local::now(),
            Some(Local::now()),
        );
        let entry_clone = entry.clone();

        let mut mock_repo = MockWorklogRepository::new();
        mock_repo
            .expect_find_by_id()
            .with(eq("id-1"))
            .times(1)
            .returning(move |_| Ok(Some(entry_clone.clone())));
        mock_repo.expect_update().times(1).returning(|_| Ok(()));

        let mock_tracker = MockIssueTrackerClient::new();
        let service = WorklogService::new(Arc::new(mock_repo), Arc::new(mock_tracker));

        let result = service
            .update_comment("id-1", Some("New comment".to_string()))
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_update_comment_not_found() {
        let mut mock_repo = MockWorklogRepository::new();
        mock_repo
            .expect_find_by_id()
            .times(1)
            .returning(|_| Ok(None));

        let mock_tracker = MockIssueTrackerClient::new();
        let service = WorklogService::new(Arc::new(mock_repo), Arc::new(mock_tracker));

        let result = service
            .update_comment("nonexistent", Some("Test".to_string()))
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_update_issue_key_success() {
        let entry = create_test_entry(
            Some("id-1"),
            Some("PROJ-1"),
            Local::now(),
            Some(Local::now()),
        );
        let entry_clone = entry.clone();

        let mut mock_repo = MockWorklogRepository::new();
        mock_repo
            .expect_find_by_id()
            .times(1)
            .returning(move |_| Ok(Some(entry_clone.clone())));
        mock_repo.expect_update().times(1).returning(|_| Ok(()));

        let mock_tracker = MockIssueTrackerClient::new();
        let service = WorklogService::new(Arc::new(mock_repo), Arc::new(mock_tracker));

        let result = service
            .update_issue_key("id-1", Some("PROJ-2".to_string()))
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_update_tag_add() {
        let entry = create_test_entry(
            Some("id-1"),
            Some("PROJ-1"),
            Local::now(),
            Some(Local::now()),
        );
        let entry_clone = entry.clone();

        let mut mock_repo = MockWorklogRepository::new();
        mock_repo
            .expect_find_by_id()
            .times(1)
            .returning(move |_| Ok(Some(entry_clone.clone())));
        mock_repo.expect_update().times(1).returning(|_| Ok(()));

        let mock_tracker = MockIssueTrackerClient::new();
        let service = WorklogService::new(Arc::new(mock_repo), Arc::new(mock_tracker));

        let result = service.update_tag("id-1", "urgent", true).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_update_tag_remove() {
        let mut entry = create_test_entry(
            Some("id-1"),
            Some("PROJ-1"),
            Local::now(),
            Some(Local::now()),
        );
        entry.tags = vec!["urgent".to_string()];
        let entry_clone = entry.clone();

        let mut mock_repo = MockWorklogRepository::new();
        mock_repo
            .expect_find_by_id()
            .times(1)
            .returning(move |_| Ok(Some(entry_clone.clone())));
        mock_repo.expect_update().times(1).returning(|_| Ok(()));

        let mock_tracker = MockIssueTrackerClient::new();
        let service = WorklogService::new(Arc::new(mock_repo), Arc::new(mock_tracker));

        let result = service.update_tag("id-1", "urgent", false).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_find_all_success() {
        let entries = vec![
            create_test_entry(
                Some("id-1"),
                Some("PROJ-1"),
                Local::now(),
                Some(Local::now()),
            ),
            create_test_entry(
                Some("id-2"),
                Some("PROJ-2"),
                Local::now(),
                Some(Local::now()),
            ),
        ];
        let entries_clone = entries.clone();

        let mut mock_repo = MockWorklogRepository::new();
        mock_repo
            .expect_find_all()
            .times(1)
            .returning(move || Ok(entries_clone.clone()));

        let mock_tracker = MockIssueTrackerClient::new();
        let service = WorklogService::new(Arc::new(mock_repo), Arc::new(mock_tracker));

        let result = service.find_all().await.unwrap();

        assert_eq!(result.len(), 2);
    }

    #[tokio::test]
    async fn test_find_after_success() {
        let now = Local::now();
        let entries = vec![create_test_entry(
            Some("id-1"),
            Some("PROJ-1"),
            now,
            Some(now),
        )];
        let entries_clone = entries.clone();

        let mut mock_repo = MockWorklogRepository::new();
        mock_repo
            .expect_find_after()
            .times(1)
            .returning(move |_| Ok(entries_clone.clone()));

        let mock_tracker = MockIssueTrackerClient::new();
        let service = WorklogService::new(Arc::new(mock_repo), Arc::new(mock_tracker));

        let result = service.find_after(Utc::now()).await.unwrap();

        assert_eq!(result.len(), 1);
    }

    #[tokio::test]
    async fn test_find_by_issue_success() {
        let entries = vec![
            create_test_entry(
                Some("id-1"),
                Some("PROJ-1"),
                Local::now(),
                Some(Local::now()),
            ),
            create_test_entry(
                Some("id-2"),
                Some("PROJ-1"),
                Local::now(),
                Some(Local::now()),
            ),
        ];
        let entries_clone = entries.clone();

        let mut mock_repo = MockWorklogRepository::new();
        mock_repo
            .expect_find_by_issue()
            .with(eq("PROJ-1"))
            .times(1)
            .returning(move |_| Ok(entries_clone.clone()));

        let mock_tracker = MockIssueTrackerClient::new();
        let service = WorklogService::new(Arc::new(mock_repo), Arc::new(mock_tracker));

        let result = service.find_by_issue("PROJ-1").await.unwrap();

        assert_eq!(result.len(), 2);
    }

    #[tokio::test]
    async fn test_get_total_time_for_issue() {
        let mut entry1 = create_test_entry(
            Some("id-1"),
            Some("PROJ-1"),
            Local::now(),
            Some(Local::now()),
        );
        entry1.time_spent_seconds = Some(3600); // 1 hour

        let mut entry2 = create_test_entry(
            Some("id-2"),
            Some("PROJ-1"),
            Local::now(),
            Some(Local::now()),
        );
        entry2.time_spent_seconds = Some(7200); // 2 hours

        let entries = vec![entry1, entry2];
        let entries_clone = entries.clone();

        let mut mock_repo = MockWorklogRepository::new();
        mock_repo
            .expect_find_by_issue()
            .times(1)
            .returning(move |_| Ok(entries_clone.clone()));

        let mock_tracker = MockIssueTrackerClient::new();
        let service = WorklogService::new(Arc::new(mock_repo), Arc::new(mock_tracker));

        let result = service.get_total_time_for_issue("PROJ-1").await.unwrap();

        assert_eq!(result.num_seconds(), 10800); // 3 hours total
    }

    #[tokio::test]
    async fn test_delete_success() {
        let mut mock_repo = MockWorklogRepository::new();
        mock_repo
            .expect_delete()
            .with(eq("id-1"))
            .times(1)
            .returning(|_| Ok(()));

        let mock_tracker = MockIssueTrackerClient::new();
        let service = WorklogService::new(Arc::new(mock_repo), Arc::new(mock_tracker));

        let result = service.delete("id-1").await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_find_by_id_found() {
        let entry = create_test_entry(
            Some("id-1"),
            Some("PROJ-1"),
            Local::now(),
            Some(Local::now()),
        );
        let entry_clone = entry.clone();

        let mut mock_repo = MockWorklogRepository::new();
        mock_repo
            .expect_find_by_id()
            .with(eq("id-1"))
            .times(1)
            .returning(move |_| Ok(Some(entry_clone.clone())));

        let mock_tracker = MockIssueTrackerClient::new();
        let service = WorklogService::new(Arc::new(mock_repo), Arc::new(mock_tracker));

        let result = service.find_by_id("id-1").await.unwrap();

        assert!(result.is_some());
    }

    #[tokio::test]
    async fn test_find_by_id_not_found() {
        let mut mock_repo = MockWorklogRepository::new();
        mock_repo
            .expect_find_by_id()
            .times(1)
            .returning(|_| Ok(None));

        let mock_tracker = MockIssueTrackerClient::new();
        let service = WorklogService::new(Arc::new(mock_repo), Arc::new(mock_tracker));

        let result = service.find_by_id("nonexistent").await.unwrap();

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_update_entry_success() {
        let entry = create_test_entry(
            Some("id-1"),
            Some("PROJ-1"),
            Local::now(),
            Some(Local::now()),
        );

        let mut mock_repo = MockWorklogRepository::new();
        mock_repo.expect_update().times(1).returning(|_| Ok(()));

        let mock_tracker = MockIssueTrackerClient::new();
        let service = WorklogService::new(Arc::new(mock_repo), Arc::new(mock_tracker));

        let result = service.update_entry(&entry).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_add_entry_success() {
        let entry = create_test_entry(None, Some("PROJ-1"), Local::now(), Some(Local::now()));

        let mut mock_repo = MockWorklogRepository::new();
        mock_repo
            .expect_add()
            .times(1)
            .returning(|_| Ok("new-id".to_string()));

        let mock_tracker = MockIssueTrackerClient::new();
        let service = WorklogService::new(Arc::new(mock_repo), Arc::new(mock_tracker));

        let result = service.add_entry(&entry).await.unwrap();

        assert_eq!(result, "new-id");
    }

    #[tokio::test]
    async fn test_add_and_sync_success() {
        let entry = create_test_entry(None, Some("PROJ-1"), Local::now(), Some(Local::now()));

        let mut mock_repo = MockWorklogRepository::new();
        mock_repo
            .expect_add()
            .times(1)
            .returning(|_| Ok("local-123".to_string()));

        let mut mock_tracker = MockIssueTrackerClient::new();
        mock_tracker
            .expect_sync_worklog()
            .times(1)
            .returning(|_| Ok("provider-456".to_string()));

        let service = WorklogService::new(Arc::new(mock_repo), Arc::new(mock_tracker));

        let result = service.add_and_sync(&entry).await.unwrap();

        assert_eq!(result.id, Some("local-123".to_string()));
        assert_eq!(result.provider_worklog_id, Some("provider-456".to_string()));
        assert!(result.synced_to_provider);
    }

    #[tokio::test]
    async fn test_add_and_sync_tracker_failure() {
        let entry = create_test_entry(None, Some("PROJ-1"), Local::now(), Some(Local::now()));

        let mock_repo = MockWorklogRepository::new();

        let mut mock_tracker = MockIssueTrackerClient::new();
        mock_tracker
            .expect_sync_worklog()
            .times(1)
            .returning(|_| Err(WorklogError::IssueTrackerError("Sync failed".to_string())));

        let service = WorklogService::new(Arc::new(mock_repo), Arc::new(mock_tracker));

        let result = service.add_and_sync(&entry).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Sync failed"));
    }

    #[tokio::test]
    async fn test_delete_and_sync_success() {
        let mut entry = create_test_entry(
            Some("id-1"),
            Some("PROJ-1"),
            Local::now(),
            Some(Local::now()),
        );
        entry.synced_to_provider = true;
        entry.provider_worklog_id = Some("provider-123".to_string());
        let entry_clone = entry.clone();

        let mut mock_repo = MockWorklogRepository::new();
        mock_repo
            .expect_find_by_id()
            .with(eq("id-1"))
            .times(1)
            .returning(move |_| Ok(Some(entry_clone.clone())));
        mock_repo
            .expect_delete()
            .with(eq("id-1"))
            .times(1)
            .returning(|_| Ok(()));

        let mut mock_tracker = MockIssueTrackerClient::new();
        mock_tracker
            .expect_delete_worklog()
            .with(eq("PROJ-1"), eq("provider-123"))
            .times(1)
            .returning(|_, _| Ok(()));

        let service = WorklogService::new(Arc::new(mock_repo), Arc::new(mock_tracker));

        let result = service.delete_and_sync("id-1").await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_delete_and_sync_not_synced() {
        let entry = create_test_entry(
            Some("id-1"),
            Some("PROJ-1"),
            Local::now(),
            Some(Local::now()),
        );
        // Not synced: synced_to_provider = false, no provider_worklog_id
        let entry_clone = entry.clone();

        let mut mock_repo = MockWorklogRepository::new();
        mock_repo
            .expect_find_by_id()
            .times(1)
            .returning(move |_| Ok(Some(entry_clone.clone())));
        mock_repo.expect_delete().times(1).returning(|_| Ok(()));

        let mut mock_tracker = MockIssueTrackerClient::new();
        mock_tracker.expect_delete_worklog().times(0); // Should not delete from tracker

        let service = WorklogService::new(Arc::new(mock_repo), Arc::new(mock_tracker));

        let result = service.delete_and_sync("id-1").await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_delete_and_sync_not_found() {
        let mut mock_repo = MockWorklogRepository::new();
        mock_repo
            .expect_find_by_id()
            .times(1)
            .returning(|_| Ok(None));

        let mock_tracker = MockIssueTrackerClient::new();
        let service = WorklogService::new(Arc::new(mock_repo), Arc::new(mock_tracker));

        let result = service.delete_and_sync("nonexistent").await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[tokio::test]
    async fn test_get_worklogs_for_issue_success() {
        let entries = vec![
            create_test_entry(
                Some("id-1"),
                Some("PROJ-1"),
                Local::now(),
                Some(Local::now()),
            ),
            create_test_entry(
                Some("id-2"),
                Some("PROJ-1"),
                Local::now(),
                Some(Local::now()),
            ),
        ];
        let entries_clone = entries.clone();

        let mock_repo = MockWorklogRepository::new();

        let mut mock_tracker = MockIssueTrackerClient::new();
        mock_tracker
            .expect_get_worklogs_for_issue()
            .with(eq("PROJ-1"), always())
            .times(1)
            .returning(move |_, _| Ok(entries_clone.clone()));

        let service = WorklogService::new(Arc::new(mock_repo), Arc::new(mock_tracker));

        let result = service
            .get_worklogs_for_issue("PROJ-1", Utc::now())
            .await
            .unwrap();

        assert_eq!(result.len(), 2);
    }

    #[tokio::test]
    async fn test_get_worklogs_for_issue_tracker_error() {
        let mock_repo = MockWorklogRepository::new();

        let mut mock_tracker = MockIssueTrackerClient::new();
        mock_tracker
            .expect_get_worklogs_for_issue()
            .times(1)
            .returning(|_, _| Err(WorklogError::IssueTrackerError("API error".to_string())));

        let service = WorklogService::new(Arc::new(mock_repo), Arc::new(mock_tracker));

        let result = service.get_worklogs_for_issue("PROJ-1", Utc::now()).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("API error"));
    }
}
