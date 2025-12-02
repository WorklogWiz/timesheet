//! Sync service for Git-like bidirectional synchronization
//!
//! This service orchestrates three-way sync operations:
//! - Fetch: Download from remote without modifying local
//! - Compare: Detect sync states and conflicts
//! - Merge: Auto-apply non-conflicting changes
//! - Push: Upload local changes to remote

use crate::domain::{RemoteWorklog, SyncConflict, SyncResult, SyncState, WorklogEntry};
use crate::error::{WorklogError, WorklogResult};
use crate::traits::{IssueRepository, IssueTrackerClient, WorklogRepository};
use chrono::{Days, Local, Utc};
use log::{debug, info, warn};
use std::collections::HashMap;
use std::sync::Arc;

/// Service for managing bidirectional sync operations
pub struct SyncService {
    worklog_repository: Arc<dyn WorklogRepository>,
    issue_repository: Arc<dyn IssueRepository>,
    issue_tracker: Arc<dyn IssueTrackerClient>,
}

impl SyncService {
    /// Create a new sync service
    ///
    /// # Arguments
    /// * `worklog_repository` - The local worklog repository
    /// * `issue_repository` - The local issue repository
    /// * `issue_tracker` - The remote issue tracker client
    #[must_use]
    pub fn new(
        worklog_repository: Arc<dyn WorklogRepository>,
        issue_repository: Arc<dyn IssueRepository>,
        issue_tracker: Arc<dyn IssueTrackerClient>,
    ) -> Self {
        Self {
            worklog_repository,
            issue_repository,
            issue_tracker,
        }
    }

    /// Fetch worklogs from remote without modifying local database
    ///
    /// Downloads worklogs from the issue tracker for all issues that exist
    /// locally, but doesn't apply any changes yet.
    ///
    /// # Arguments
    /// * `days_back` - How many days back to fetch (default: 30)
    ///
    /// # Returns
    /// Vector of remote worklogs fetched from the issue tracker
    ///
    /// # Errors
    /// Returns `WorklogError` if date calculation fails or issue tracker queries fail
    pub async fn fetch_from_remote(
        &self,
        days_back: Option<i64>,
    ) -> WorklogResult<Vec<RemoteWorklog>> {
        let days = days_back.unwrap_or(30);
        let since = Local::now()
            .checked_sub_days(Days::new(days.unsigned_abs()))
            .ok_or_else(|| WorklogError::ValidationError("Invalid days_back value".to_string()))?;

        info!("Fetching worklogs from remote (last {days} days)");

        // Get all local issues that have worklogs
        let local_issues = self.issue_repository.find_keys_with_worklogs().await?;
        debug!("Found {} local issues with worklogs", local_issues.len());

        let mut remote_worklogs = Vec::new();

        // Fetch worklogs for each issue
        for issue_key in &local_issues {
            debug!("Fetching worklogs for issue: {issue_key}");

            match self
                .issue_tracker
                .get_worklogs_for_issue(issue_key, since.into())
                .await
            {
                Ok(entries) => {
                    debug!("  Found {} worklogs for {issue_key}", entries.len());

                    // Convert WorklogEntry to RemoteWorklog
                    for entry in entries {
                        if let Some(provider_id) = entry.provider_worklog_id {
                            remote_worklogs.push(RemoteWorklog {
                                provider_worklog_id: provider_id.clone(),
                                issue_key: entry.issue_key.unwrap_or_else(|| issue_key.clone()),
                                started_at: entry.started_at.with_timezone(&Utc),
                                stopped_at: entry
                                    .stopped_at
                                    .map(|dt| dt.with_timezone(&Utc))
                                    .ok_or_else(|| {
                                        WorklogError::ValidationError(format!(
                                            "Remote worklog {provider_id} has no stopped_at"
                                        ))
                                    })?,
                                time_spent_seconds: entry.time_spent_seconds.ok_or_else(|| {
                                    WorklogError::ValidationError(format!(
                                        "Remote worklog {provider_id} has no time_spent_seconds"
                                    ))
                                })?,
                                comment: entry.comment,
                                updated_at: entry.updated_at.with_timezone(&Utc),
                            });
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to fetch worklogs for {issue_key}: {e}");
                    // Continue with other issues
                }
            }
        }

        info!("Fetched {} worklogs from remote", remote_worklogs.len());
        Ok(remote_worklogs)
    }

    /// Compare local and remote worklogs to detect sync states
    ///
    /// # Arguments
    /// * `remote_worklogs` - Worklogs fetched from remote
    ///
    /// # Returns
    /// Vector of sync conflicts with detected states
    ///
    /// # Errors
    /// Returns `WorklogError` if repository operations fail
    pub async fn compare(
        &self,
        remote_worklogs: Vec<RemoteWorklog>,
    ) -> WorklogResult<Vec<SyncConflict>> {
        info!("Comparing local and remote worklogs");

        let mut conflicts = Vec::new();

        // Build a map of remote worklogs by provider ID
        let remote_map: HashMap<String, RemoteWorklog> = remote_worklogs
            .into_iter()
            .map(|rw| (rw.provider_worklog_id.clone(), rw))
            .collect();

        // Get all local worklogs (including soft-deleted) that have provider IDs
        let all_local = self.worklog_repository.find_all().await?;
        let mut local_map: HashMap<String, WorklogEntry> = HashMap::new();
        let mut local_only: Vec<WorklogEntry> = Vec::new();

        // Track duplicates but DON'T delete them during sync!
        // Deleting during sync causes those deletions to propagate to remote.
        // Duplicates should be cleaned up by a separate maintenance operation.
        for entry in all_local {
            if let Some(provider_id) = &entry.provider_worklog_id {
                if let Some(existing) = local_map.get(provider_id) {
                    // Duplicate detected! Warn but keep the newer one in the map
                    warn!(
                        "Duplicate detected for provider_worklog_id={}: local_id={} vs {} (keeping newer for sync comparison)",
                        provider_id,
                        existing.id.as_ref().unwrap_or(&"unknown".to_string()),
                        entry.id.as_ref().unwrap_or(&"unknown".to_string())
                    );

                    // Only update the map if this entry is newer
                    if entry.updated_at > existing.updated_at {
                        local_map.insert(provider_id.clone(), entry);
                    }
                    // Otherwise keep the existing (newer) one
                } else {
                    // No duplicate, just insert
                    local_map.insert(provider_id.clone(), entry);
                }
            } else {
                // No provider ID = never synced, needs to be uploaded
                local_only.push(entry);
            }
        }

        // Find local worklogs that also exist in soft-deleted state
        let provider_ids: Vec<String> = local_map.keys().cloned().collect();
        for provider_id in provider_ids {
            if let Ok(Some(entry)) = self
                .worklog_repository
                .find_by_provider_id(&provider_id)
                .await
            {
                if entry.deleted_at.is_some() {
                    local_map.insert(provider_id, entry);
                }
            }
        }

        // Check all remote worklogs against local
        for (provider_id, remote) in &remote_map {
            let local = local_map.get(provider_id);

            let state = self.detect_sync_state(local, Some(remote)).await?;

            conflicts.push(SyncConflict {
                provider_worklog_id: provider_id.clone(),
                local: local.cloned(),
                remote: Some(remote.clone()),
                state,
            });
        }

        // Check local-only entries (not in remote)
        for (provider_id, local) in &local_map {
            if !remote_map.contains_key(provider_id) {
                let state = self.detect_sync_state(Some(local), None).await?;

                conflicts.push(SyncConflict {
                    provider_worklog_id: provider_id.clone(),
                    local: Some(local.clone()),
                    remote: None,
                    state,
                });
            }
        }

        // Add unsynced local entries (no provider_worklog_id yet)
        for local in local_only {
            let local_id = local.id.as_ref().unwrap_or(&"unknown".to_string()).clone();
            let state = self.detect_sync_state(Some(&local), None).await?;

            conflicts.push(SyncConflict {
                provider_worklog_id: format!("local-{local_id}"), // Temporary ID
                local: Some(local),
                remote: None,
                state,
            });
        }

        info!("Found {} sync items to process", conflicts.len());
        Ok(conflicts)
    }

    /// Detect the sync state between local and remote entries
    async fn detect_sync_state(
        &self,
        local: Option<&WorklogEntry>,
        remote: Option<&RemoteWorklog>,
    ) -> WorklogResult<SyncState> {
        match (local, remote) {
            // Both exist - check which is newer or if conflict
            (Some(local_entry), Some(remote_entry)) => {
                // Check if local was deleted
                if local_entry.is_deleted() {
                    return Ok(SyncState::DeletedLocal);
                }

                // Compare timestamps
                let local_updated = local_entry.updated_at.with_timezone(&Utc);
                let remote_updated = remote_entry.updated_at;
                let last_synced = local_entry.last_synced_at.map(|dt| dt.with_timezone(&Utc));

                match last_synced {
                    Some(synced_at) => {
                        let local_modified = local_updated > synced_at;
                        let remote_modified = remote_updated > synced_at;

                        // Debug logging to understand the issue
                        if remote_modified {
                            debug!(
                                "Provider {}: remote_modified=true (remote: {}, synced: {})",
                                local_entry
                                    .provider_worklog_id
                                    .as_deref()
                                    .unwrap_or("unknown"),
                                remote_updated,
                                synced_at
                            );
                        }

                        if local_modified && remote_modified {
                            Ok(SyncState::Conflict)
                        } else if local_modified {
                            Ok(SyncState::LocalNewer)
                        } else if remote_modified {
                            Ok(SyncState::RemoteNewer)
                        } else {
                            Ok(SyncState::InSync)
                        }
                    }
                    None => {
                        // Never synced - check if content matches
                        if Self::entries_match(local_entry, remote_entry) {
                            Ok(SyncState::InSync)
                        } else {
                            Ok(SyncState::Conflict)
                        }
                    }
                }
            }

            // Only local exists
            (Some(local_entry), None) => {
                if local_entry.is_deleted() {
                    // Deleted locally and not on remote - already in sync
                    Ok(SyncState::InSync)
                } else if local_entry.provider_worklog_id.is_some() {
                    // Has provider ID but not in remote - was deleted remotely
                    Ok(SyncState::DeletedRemote)
                } else {
                    // No provider ID - local-only entry that needs upload
                    Ok(SyncState::LocalOnly)
                }
            }

            // Only remote exists
            (None, Some(remote_entry)) => {
                // Check if tombstoned (was deleted locally)
                if self
                    .worklog_repository
                    .is_tombstoned(&remote_entry.provider_worklog_id)
                    .await?
                {
                    Ok(SyncState::DeletedLocal)
                } else {
                    Ok(SyncState::RemoteOnly)
                }
            }

            // Neither exists (shouldn't happen)
            (None, None) => Ok(SyncState::InSync),
        }
    }

    /// Check if local and remote entries have matching content
    fn entries_match(local: &WorklogEntry, remote: &RemoteWorklog) -> bool {
        let local_started = local.started_at.with_timezone(&Utc);
        let local_stopped = local
            .stopped_at
            .map_or_else(Utc::now, |dt| dt.with_timezone(&Utc));

        local_started == remote.started_at
            && local_stopped == remote.stopped_at
            && local.time_spent_seconds == Some(remote.time_spent_seconds)
            && local.comment == remote.comment
    }

    /// Auto-merge non-conflicting changes
    ///
    /// Applies changes that don't require user intervention:
    /// - Downloads new remote entries
    /// - Updates entries modified only on remote
    /// - Uploads entries modified only locally
    /// - Propagates deletions
    ///
    /// # Arguments
    /// * `conflicts` - Sync conflicts detected by `compare()`
    ///
    /// # Returns
    /// Sync result with statistics and remaining conflicts
    ///
    /// # Errors
    /// Returns `WorklogError` if repository or issue tracker operations fail
    pub async fn auto_merge(&self, conflicts: Vec<SyncConflict>) -> WorklogResult<SyncResult> {
        info!("Auto-merging {} sync items", conflicts.len());

        let mut result = SyncResult::new();

        for conflict in conflicts {
            if conflict.state.needs_user_resolution() {
                result.conflicts += 1;
                result.conflict_details.push(conflict.clone());
                continue;
            }

            let provider_worklog_id = conflict.provider_worklog_id.clone();
            match self.apply_sync_action(conflict).await {
                Ok(action) => match action {
                    SyncAction::Added => result.added += 1,
                    SyncAction::Updated => result.updated += 1,
                    SyncAction::Uploaded => result.uploaded += 1,
                    SyncAction::DeletedLocal => result.deleted_local += 1,
                    SyncAction::DeletedRemote => result.deleted_remote += 1,
                    SyncAction::None => {}
                },
                Err(e) => {
                    warn!("Failed to apply sync action for {provider_worklog_id}: {e}");
                    // Continue with other entries
                }
            }
        }

        info!(
            "Auto-merge complete: {} added, {} updated, {} uploaded, {} conflicts",
            result.added, result.updated, result.uploaded, result.conflicts
        );

        Ok(result)
    }

    /// Apply a sync action for a single conflict
    #[allow(clippy::too_many_lines)]
    async fn apply_sync_action(&self, conflict: SyncConflict) -> WorklogResult<SyncAction> {
        match conflict.state {
            SyncState::InSync => Ok(SyncAction::None),

            SyncState::RemoteOnly => {
                // Add from remote (but check for duplicates first!)
                let remote = conflict
                    .remote
                    .ok_or_else(|| WorklogError::ValidationError("Missing remote entry".into()))?;

                // Check if it already exists (shouldn't happen, but be safe)
                if let Ok(Some(_)) = self
                    .worklog_repository
                    .find_by_provider_id(&remote.provider_worklog_id)
                    .await
                {
                    debug!(
                        "Skipping RemoteOnly entry {} - already exists locally",
                        remote.provider_worklog_id
                    );
                    return Ok(SyncAction::None);
                }

                let entry = remote.to_worklog_entry();
                self.worklog_repository.add(&entry).await?;
                Ok(SyncAction::Added)
            }

            SyncState::LocalOnly => {
                // Upload to remote
                let local = conflict
                    .local
                    .ok_or_else(|| WorklogError::ValidationError("Missing local entry".into()))?;
                let provider_id = self.issue_tracker.sync_worklog(&local).await?;

                // Update local with provider ID and mark as synced
                let mut updated = local;
                updated.mark_synced(Some(provider_id));
                self.worklog_repository.update(&updated).await?;
                Ok(SyncAction::Uploaded)
            }

            SyncState::RemoteNewer => {
                // Update local from remote
                let mut local = conflict
                    .local
                    .ok_or_else(|| WorklogError::ValidationError("Missing local entry".into()))?;
                let remote = conflict
                    .remote
                    .ok_or_else(|| WorklogError::ValidationError("Missing remote entry".into()))?;

                // Update fields from remote but preserve local metadata
                local.started_at = remote.started_at.with_timezone(&chrono::Local);
                local.stopped_at = Some(remote.stopped_at.with_timezone(&chrono::Local));
                local.time_spent_seconds = Some(remote.time_spent_seconds);
                local.comment.clone_from(&remote.comment);
                local.updated_at = remote.updated_at.with_timezone(&chrono::Local);
                local.last_synced_at = Some(chrono::Local::now());
                // Keep: id, issue_key, tags, created_at, version, synced_to_provider, provider_worklog_id, deleted_at

                self.worklog_repository.update(&local).await?;
                Ok(SyncAction::Updated)
            }

            SyncState::LocalNewer => {
                // Upload to remote
                let local = conflict
                    .local
                    .ok_or_else(|| WorklogError::ValidationError("Missing local entry".into()))?;

                if let Some(issue_key) = &local.issue_key {
                    if let Some(provider_id) = &local.provider_worklog_id {
                        // Delete old worklog and create new one (update not supported by all trackers)
                        self.issue_tracker
                            .delete_worklog(issue_key, provider_id)
                            .await?;
                    }
                }

                let provider_id = self.issue_tracker.sync_worklog(&local).await?;

                // Update local with new provider ID and mark as synced
                let mut updated = local;
                updated.mark_synced(Some(provider_id));
                self.worklog_repository.update(&updated).await?;
                Ok(SyncAction::Uploaded)
            }

            SyncState::DeletedLocal => {
                // Delete from remote
                // Use remote entry to get issue_key (local might be tombstoned without full data)
                let issue_key = conflict
                    .local
                    .as_ref()
                    .and_then(|local| local.issue_key.as_ref())
                    .or_else(|| conflict.remote.as_ref().map(|remote| &remote.issue_key));

                if let Some(issue_key) = issue_key {
                    debug!(
                        "Deleting worklog {} from remote issue {}",
                        conflict.provider_worklog_id, issue_key
                    );
                    self.issue_tracker
                        .delete_worklog(issue_key, &conflict.provider_worklog_id)
                        .await?;
                } else {
                    warn!(
                        "Cannot delete worklog {} from remote: no issue key available",
                        conflict.provider_worklog_id
                    );
                }
                Ok(SyncAction::DeletedRemote)
            }

            SyncState::DeletedRemote => {
                // Soft delete locally
                let local = conflict
                    .local
                    .ok_or_else(|| WorklogError::ValidationError("Missing local entry".into()))?;

                if let Some(id) = &local.id {
                    self.worklog_repository.delete(id).await?;
                }
                Ok(SyncAction::DeletedLocal)
            }

            SyncState::Conflict => {
                // Should not happen - conflicts are filtered out before apply_sync_action
                Err(WorklogError::ValidationError(
                    "Cannot auto-merge conflict".into(),
                ))
            }
        }
    }

    /// Full synchronization: fetch, compare, and auto-merge
    ///
    /// This is a convenience method that combines all sync operations.
    ///
    /// # Arguments
    /// * `days_back` - How many days back to sync (default: 30)
    ///
    /// # Returns
    /// Sync result with statistics and any remaining conflicts
    ///
    /// # Errors
    /// Returns `WorklogError` if any sync operation fails
    pub async fn sync(&self, days_back: Option<i64>) -> WorklogResult<SyncResult> {
        info!("Starting full synchronization");

        // Clean up expired tombstones first
        let cleaned = self.worklog_repository.cleanup_tombstones().await?;
        if cleaned > 0 {
            debug!("Cleaned up {cleaned} expired tombstones");
        }

        // Fetch from remote
        let remote_worklogs = self.fetch_from_remote(days_back).await?;

        // Compare
        let conflicts = self.compare(remote_worklogs).await?;

        // Auto-merge
        let result = self.auto_merge(conflicts).await?;

        info!("Synchronization complete");
        Ok(result)
    }
}

/// Internal enum for tracking sync actions
enum SyncAction {
    Added,
    Updated,
    Uploaded,
    DeletedLocal,
    DeletedRemote,
    None,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{RemoteWorklog, SyncState, WorklogEntry};
    use crate::traits::{MockIssueRepository, MockIssueTrackerClient, MockWorklogRepository};
    use chrono::{DateTime, Duration, Local, Utc};
    use mockall::predicate::*;
    use std::sync::Arc;

    // Helper to create a WorklogEntry
    fn create_local_entry(
        id: &str,
        provider_id: Option<&str>,
        updated_at: DateTime<Local>,
        last_synced_at: Option<DateTime<Local>>,
        deleted_at: Option<DateTime<Local>>,
        issue_key: Option<&str>,
        stopped_at: Option<DateTime<Local>>,
    ) -> WorklogEntry {
        WorklogEntry {
            id: Some(id.to_string()),
            issue_key: issue_key.map(std::string::ToString::to_string),
            started_at: updated_at - Duration::hours(1),
            stopped_at,
            comment: Some("test".to_string()),
            tags: vec![],
            time_spent_seconds: Some(3600),
            synced_to_provider: provider_id.is_some(),
            provider_worklog_id: provider_id.map(std::string::ToString::to_string),
            created_at: updated_at - Duration::hours(2),
            updated_at,
            deleted_at,
            last_synced_at,
            version: 1,
            has_conflict: false,
        }
    }

    // Helper to create a RemoteWorklog
    fn create_remote_entry(
        provider_id: &str,
        updated_at: DateTime<Utc>,
        issue_key: &str,
    ) -> RemoteWorklog {
        RemoteWorklog {
            provider_worklog_id: provider_id.to_string(),
            issue_key: issue_key.to_string(),
            started_at: updated_at - Duration::hours(1),
            stopped_at: updated_at,
            comment: Some("test".to_string()),
            time_spent_seconds: 3600,
            updated_at,
        }
    }

    // Unified helper to create SyncService with configurable mocks
    fn create_sync_service_with_mocks<FW, FI, FT>(
        setup_worklog: FW,
        setup_issue: FI,
        setup_tracker: FT,
    ) -> SyncService
    where
        FW: FnOnce(&mut MockWorklogRepository),
        FI: FnOnce(&mut MockIssueRepository),
        FT: FnOnce(&mut MockIssueTrackerClient),
    {
        let mut mock_worklog_repo = MockWorklogRepository::new();
        setup_worklog(&mut mock_worklog_repo);

        let mut mock_issue_repo = MockIssueRepository::new();
        setup_issue(&mut mock_issue_repo);

        let mut mock_issue_tracker = MockIssueTrackerClient::new();
        setup_tracker(&mut mock_issue_tracker);

        SyncService::new(
            Arc::new(mock_worklog_repo),
            Arc::new(mock_issue_repo),
            Arc::new(mock_issue_tracker),
        )
    }

    // Helper to create a basic SyncService with empty mocks
    fn create_sync_service() -> SyncService {
        create_sync_service_with_mocks(|_| {}, |_| {}, |_| {})
    }

    // Helper to create in-sync local and remote entries
    fn create_in_sync_pair(
        id: &str,
        provider_id: &str,
        issue_key: &str,
    ) -> (WorklogEntry, RemoteWorklog) {
        let now = Local::now();
        let now_utc = now.with_timezone(&Utc);
        let local = create_local_entry(
            id,
            Some(provider_id),
            now,
            Some(now),
            None,
            Some(issue_key),
            Some(now),
        );
        let remote = create_remote_entry(provider_id, now_utc, issue_key);
        (local, remote)
    }

    // Helper to create a conflict scenario (both modified after last sync)
    fn create_conflict_pair(
        id: &str,
        provider_id: &str,
        issue_key: &str,
    ) -> (WorklogEntry, RemoteWorklog) {
        let now = Local::now();
        let now_utc = now.with_timezone(&Utc);
        let synced_at = now - Duration::hours(1);
        // Both modified after sync
        let local = create_local_entry(
            id,
            Some(provider_id),
            now - Duration::minutes(5),
            Some(synced_at),
            None,
            Some(issue_key),
            Some(now - Duration::minutes(5)),
        );
        let remote = create_remote_entry(provider_id, now_utc, issue_key);
        (local, remote)
    }

    // Helper to create a SyncConflict
    fn create_sync_conflict(
        provider_id: &str,
        local: Option<WorklogEntry>,
        remote: Option<RemoteWorklog>,
        state: SyncState,
    ) -> SyncConflict {
        SyncConflict {
            provider_worklog_id: provider_id.to_string(),
            local,
            remote,
            state,
        }
    }

    #[test]
    fn test_sync_state_needs_resolution() {
        assert!(SyncState::Conflict.needs_user_resolution());
        assert!(!SyncState::InSync.needs_user_resolution());
        assert!(!SyncState::LocalOnly.needs_user_resolution());
        assert!(!SyncState::RemoteOnly.needs_user_resolution());
    }

    #[test]
    fn test_sync_result_default() {
        let result = SyncResult::default();
        assert_eq!(result.added, 0);
        assert!(!result.has_changes());
    }

    #[tokio::test]
    async fn test_detect_sync_state_in_sync() {
        let (local, remote) = create_in_sync_pair("l1", "p1", "ISSUE-1");
        let sync_service = create_sync_service();

        let state = sync_service
            .detect_sync_state(Some(&local), Some(&remote))
            .await
            .unwrap();
        assert_eq!(state, SyncState::InSync);
    }

    #[tokio::test]
    async fn test_detect_sync_state_local_only_without_provider_id() {
        let now = Local::now();
        let local = create_local_entry("l1", None, now, None, None, Some("ISSUE-1"), Some(now));
        let sync_service = create_sync_service();

        let state = sync_service
            .detect_sync_state(Some(&local), None)
            .await
            .unwrap();
        assert_eq!(state, SyncState::LocalOnly);
    }

    #[tokio::test]
    async fn test_detect_sync_state_deleted_remote_with_provider_id() {
        // BUG FIX TEST: Entry with provider_worklog_id but not in remote = DeletedRemote
        let now = Local::now();
        let local = create_local_entry(
            "l1",
            Some("p1"),
            now,
            Some(now),
            None,
            Some("ISSUE-1"),
            Some(now),
        );
        let sync_service = create_sync_service();

        let state = sync_service
            .detect_sync_state(Some(&local), None)
            .await
            .unwrap();
        assert_eq!(state, SyncState::DeletedRemote);
    }

    #[tokio::test]
    async fn test_detect_sync_state_remote_only() {
        let remote = create_remote_entry("p1", Utc::now(), "ISSUE-1");

        let sync_service = create_sync_service_with_mocks(
            |repo| {
                repo.expect_is_tombstoned()
                    .with(eq("p1"))
                    .returning(|_| Ok(false));
            },
            |_| {},
            |_| {},
        );

        let state = sync_service
            .detect_sync_state(None, Some(&remote))
            .await
            .unwrap();
        assert_eq!(state, SyncState::RemoteOnly);
    }

    #[tokio::test]
    async fn test_detect_sync_state_local_newer() {
        let now = Local::now();
        let synced_at = now - Duration::minutes(30);
        // Local modified after sync (now), remote modified BEFORE sync (now - 40min)
        let local = create_local_entry(
            "l1",
            Some("p1"),
            now,
            Some(synced_at),
            None,
            Some("ISSUE-1"),
            Some(now),
        );
        let remote = create_remote_entry(
            "p1",
            (now - Duration::minutes(40)).with_timezone(&Utc),
            "ISSUE-1",
        );
        let sync_service = create_sync_service();

        let state = sync_service
            .detect_sync_state(Some(&local), Some(&remote))
            .await
            .unwrap();
        assert_eq!(state, SyncState::LocalNewer);
    }

    #[tokio::test]
    async fn test_detect_sync_state_remote_newer() {
        let now = Local::now();
        let now_utc = now.with_timezone(&Utc);
        let synced_at = now - Duration::minutes(30);
        // Local modified BEFORE sync (now - 40min), remote modified after sync (now)
        let local = create_local_entry(
            "l1",
            Some("p1"),
            now - Duration::minutes(40),
            Some(synced_at),
            None,
            Some("ISSUE-1"),
            Some(now - Duration::minutes(40)),
        );
        let remote = create_remote_entry("p1", now_utc, "ISSUE-1");
        let sync_service = create_sync_service();

        let state = sync_service
            .detect_sync_state(Some(&local), Some(&remote))
            .await
            .unwrap();
        assert_eq!(state, SyncState::RemoteNewer);
    }

    #[tokio::test]
    async fn test_detect_sync_state_conflict() {
        let (local, remote) = create_conflict_pair("l1", "p1", "ISSUE-1");
        let sync_service = create_sync_service();

        let state = sync_service
            .detect_sync_state(Some(&local), Some(&remote))
            .await
            .unwrap();
        assert_eq!(state, SyncState::Conflict);
    }

    #[tokio::test]
    async fn test_detect_sync_state_deleted_local() {
        let now = Local::now();
        let now_utc = now.with_timezone(&Utc);
        let local = create_local_entry(
            "l1",
            Some("p1"),
            now,
            Some(now),
            Some(now), // deleted_at
            Some("ISSUE-1"),
            Some(now),
        );
        let remote = create_remote_entry("p1", now_utc, "ISSUE-1");
        let sync_service = create_sync_service();

        let state = sync_service
            .detect_sync_state(Some(&local), Some(&remote))
            .await
            .unwrap();
        assert_eq!(state, SyncState::DeletedLocal);
    }

    #[tokio::test]
    async fn test_detect_sync_state_local_soft_deleted_no_remote_in_sync() {
        let now = Local::now();
        let local = create_local_entry(
            "l1",
            Some("p1"),
            now,
            Some(now),
            Some(now), // deleted_at
            Some("ISSUE-1"),
            Some(now),
        );
        let sync_service = create_sync_service();

        let state = sync_service
            .detect_sync_state(Some(&local), None)
            .await
            .unwrap();
        // Deleted locally and not on remote = in sync
        assert_eq!(state, SyncState::InSync);
    }

    #[tokio::test]
    async fn test_detect_sync_state_tombstoned() {
        let remote = create_remote_entry("p1", Utc::now(), "ISSUE-1");

        let sync_service = create_sync_service_with_mocks(
            |repo| {
                repo.expect_is_tombstoned()
                    .with(eq("p1"))
                    .returning(|_| Ok(true)); // Is tombstoned
            },
            |_| {},
            |_| {},
        );

        let state = sync_service
            .detect_sync_state(None, Some(&remote))
            .await
            .unwrap();
        // Tombstoned = was deleted locally, remote still exists
        assert_eq!(state, SyncState::DeletedLocal);
    }

    #[tokio::test]
    async fn test_entries_match() {
        let now = Local::now();
        let now_utc = now.with_timezone(&Utc);
        let local = create_local_entry(
            "l1",
            Some("p1"),
            now,
            Some(now),
            None,
            Some("ISSUE-1"),
            Some(now),
        );
        let remote = RemoteWorklog {
            provider_worklog_id: "p1".to_string(),
            issue_key: "ISSUE-1".to_string(),
            started_at: local.started_at.with_timezone(&Utc),
            stopped_at: local.stopped_at.unwrap().with_timezone(&Utc),
            comment: local.comment.clone(),
            time_spent_seconds: local.time_spent_seconds.unwrap(),
            updated_at: now_utc,
        };

        assert!(SyncService::entries_match(&local, &remote));
    }

    #[tokio::test]
    async fn test_entries_do_not_match() {
        let now = Local::now();
        let now_utc = now.with_timezone(&Utc);
        let local = create_local_entry(
            "l1",
            Some("p1"),
            now,
            Some(now),
            None,
            Some("ISSUE-1"),
            Some(now),
        );
        let remote = RemoteWorklog {
            provider_worklog_id: "p1".to_string(),
            issue_key: "ISSUE-1".to_string(),
            started_at: local.started_at.with_timezone(&Utc) + Duration::minutes(5), // Different start
            stopped_at: local.stopped_at.unwrap().with_timezone(&Utc),
            comment: local.comment.clone(),
            time_spent_seconds: local.time_spent_seconds.unwrap(),
            updated_at: now_utc,
        };

        assert!(!SyncService::entries_match(&local, &remote));
    }

    // Integration tests for full sync workflow

    #[tokio::test]
    async fn test_compare_returns_all_conflicts() {
        let now = Local::now();
        let now_utc = now.with_timezone(&Utc);

        // Create test data: 1 in sync, 1 local-only, 1 remote-only
        let local1 = create_local_entry(
            "l1",
            Some("p1"),
            now,
            Some(now),
            None,
            Some("ISSUE-1"),
            Some(now),
        );
        let local2 = create_local_entry("l2", None, now, None, None, Some("ISSUE-2"), Some(now));
        let remote1 = create_remote_entry("p1", now_utc, "ISSUE-1");
        let remote3 = create_remote_entry("p3", now_utc, "ISSUE-3");

        let sync_service = create_sync_service_with_mocks(
            |repo| {
                repo.expect_find_all()
                    .returning(move || Ok(vec![local1.clone(), local2.clone()]));
                repo.expect_find_by_provider_id().returning(|_| Ok(None));
                repo.expect_is_tombstoned()
                    .with(eq("p3"))
                    .returning(|_| Ok(false));
            },
            |_| {},
            |_| {},
        );

        let conflicts = sync_service.compare(vec![remote1, remote3]).await.unwrap();

        // Should detect: 1 in-sync (p1), 1 remote-only (p3), 1 local-only (l2 without provider_id)
        assert_eq!(conflicts.len(), 3); // p1 (in-sync), p3 (remote-only), l2 (local-only)
    }

    #[tokio::test]
    async fn test_auto_merge_local_only_uploads() {
        let now = Local::now();
        let local = create_local_entry("l1", None, now, None, None, Some("ISSUE-1"), Some(now));
        let conflict = create_sync_conflict("l1", Some(local.clone()), None, SyncState::LocalOnly);

        let sync_service = create_sync_service_with_mocks(
            |repo| {
                repo.expect_update().returning(|_| Ok(()));
            },
            |_| {},
            |tracker| {
                tracker
                    .expect_sync_worklog()
                    .returning(|_| Ok("new_provider_id".to_string()));
            },
        );

        let result = sync_service.auto_merge(vec![conflict]).await.unwrap();

        assert_eq!(result.uploaded, 1);
        assert_eq!(result.conflicts, 0);
    }

    #[tokio::test]
    async fn test_auto_merge_remote_only_downloads() {
        let remote = create_remote_entry("p1", Utc::now(), "ISSUE-1");
        let conflict = create_sync_conflict("p1", None, Some(remote), SyncState::RemoteOnly);

        let sync_service = create_sync_service_with_mocks(
            |repo| {
                repo.expect_is_tombstoned()
                    .with(eq("p1"))
                    .returning(|_| Ok(false));
                repo.expect_find_by_provider_id()
                    .with(eq("p1"))
                    .returning(|_| Ok(None));
                repo.expect_add()
                    .returning(|_| Ok("new_local_id".to_string()));
            },
            |_| {},
            |_| {},
        );

        let result = sync_service.auto_merge(vec![conflict]).await.unwrap();

        assert_eq!(result.added, 1);
        assert_eq!(result.conflicts, 0);
    }

    #[tokio::test]
    async fn test_auto_merge_remote_newer_updates() {
        let now = Local::now();
        let now_utc = now.with_timezone(&Utc);
        let synced_at = now - Duration::minutes(30);
        let local = create_local_entry(
            "l1",
            Some("p1"),
            now - Duration::minutes(40),
            Some(synced_at),
            None,
            Some("ISSUE-1"),
            Some(now - Duration::minutes(40)),
        );
        let remote = create_remote_entry("p1", now_utc, "ISSUE-1");
        let conflict =
            create_sync_conflict("p1", Some(local), Some(remote), SyncState::RemoteNewer);

        let sync_service = create_sync_service_with_mocks(
            |repo| {
                repo.expect_update().returning(|_| Ok(()));
            },
            |_| {},
            |_| {},
        );

        let result = sync_service.auto_merge(vec![conflict]).await.unwrap();

        assert_eq!(result.updated, 1);
        assert_eq!(result.conflicts, 0);
    }

    #[tokio::test]
    async fn test_auto_merge_local_newer_uploads() {
        let now = Local::now();
        let synced_at = now - Duration::minutes(30);
        let local = create_local_entry(
            "l1",
            Some("p1"),
            now,
            Some(synced_at),
            None,
            Some("ISSUE-1"),
            Some(now),
        );
        let remote = create_remote_entry(
            "p1",
            (now - Duration::minutes(40)).with_timezone(&Utc),
            "ISSUE-1",
        );
        let conflict = create_sync_conflict("p1", Some(local), Some(remote), SyncState::LocalNewer);

        let sync_service = create_sync_service_with_mocks(
            |repo| {
                repo.expect_update().returning(|_| Ok(()));
            },
            |_| {},
            |tracker| {
                tracker
                    .expect_delete_worklog()
                    .with(eq("ISSUE-1"), eq("p1"))
                    .returning(|_, _| Ok(()));
                tracker
                    .expect_sync_worklog()
                    .returning(|_| Ok("p1".to_string()));
            },
        );

        let result = sync_service.auto_merge(vec![conflict]).await.unwrap();

        assert_eq!(result.uploaded, 1);
        assert_eq!(result.conflicts, 0);
    }

    #[tokio::test]
    async fn test_auto_merge_deleted_local_deletes_remote() {
        let now = Local::now();
        let now_utc = now.with_timezone(&Utc);
        let local = create_local_entry(
            "l1",
            Some("p1"),
            now,
            Some(now),
            Some(now), // deleted
            Some("ISSUE-1"),
            Some(now),
        );
        let remote = create_remote_entry("p1", now_utc, "ISSUE-1");
        let conflict =
            create_sync_conflict("p1", Some(local), Some(remote), SyncState::DeletedLocal);

        let sync_service = create_sync_service_with_mocks(
            |repo| {
                repo.expect_delete().with(eq("l1")).returning(|_| Ok(()));
            },
            |_| {},
            |tracker| {
                tracker
                    .expect_delete_worklog()
                    .with(eq("ISSUE-1"), eq("p1"))
                    .returning(|_, _| Ok(()));
            },
        );

        let result = sync_service.auto_merge(vec![conflict]).await.unwrap();

        assert_eq!(result.deleted_remote, 1);
        assert_eq!(result.conflicts, 0);
    }

    #[tokio::test]
    async fn test_auto_merge_deleted_remote_deletes_local() {
        let now = Local::now();
        let local = create_local_entry(
            "l1",
            Some("p1"),
            now,
            Some(now),
            None,
            Some("ISSUE-1"),
            Some(now),
        );
        let conflict = create_sync_conflict("p1", Some(local), None, SyncState::DeletedRemote);

        let sync_service = create_sync_service_with_mocks(
            |repo| {
                repo.expect_delete().with(eq("l1")).returning(|_| Ok(()));
            },
            |_| {},
            |_| {},
        );

        let result = sync_service.auto_merge(vec![conflict]).await.unwrap();

        assert_eq!(result.deleted_local, 1);
        assert_eq!(result.conflicts, 0);
    }

    #[tokio::test]
    async fn test_auto_merge_conflict_not_merged() {
        let (local, remote) = create_conflict_pair("l1", "p1", "ISSUE-1");
        let conflict = create_sync_conflict("p1", Some(local), Some(remote), SyncState::Conflict);
        let sync_service = create_sync_service();

        let result = sync_service.auto_merge(vec![conflict]).await.unwrap();

        assert_eq!(result.conflicts, 1);
        assert_eq!(result.added, 0);
        assert_eq!(result.updated, 0);
        assert_eq!(result.uploaded, 0);
    }

    #[tokio::test]
    async fn test_auto_merge_in_sync_no_action() {
        let (local, remote) = create_in_sync_pair("l1", "p1", "ISSUE-1");
        let conflict = create_sync_conflict("p1", Some(local), Some(remote), SyncState::InSync);
        let sync_service = create_sync_service();

        let result = sync_service.auto_merge(vec![conflict]).await.unwrap();

        assert_eq!(result.added, 0);
        assert_eq!(result.updated, 0);
        assert_eq!(result.uploaded, 0);
        assert_eq!(result.conflicts, 0);
    }

    #[tokio::test]
    async fn test_full_sync_workflow_everything_in_sync() {
        let sync_service = create_sync_service_with_mocks(
            |repo| {
                repo.expect_cleanup_tombstones().returning(|| Ok(0));
                repo.expect_find_all().returning(|| Ok(vec![]));
                repo.expect_find_by_provider_id().returning(|_| Ok(None));
            },
            |issue_repo| {
                issue_repo
                    .expect_find_keys_with_worklogs()
                    .returning(|| Ok(vec![]));
            },
            |tracker| {
                tracker
                    .expect_get_recently_worked_issues()
                    .returning(|| Ok(vec![]));
            },
        );

        let result = sync_service.sync(Some(30)).await.unwrap();

        assert_eq!(result.added, 0);
        assert_eq!(result.updated, 0);
        assert_eq!(result.uploaded, 0);
        assert_eq!(result.conflicts, 0);
        assert!(!result.has_changes());
    }

    // Tests for fetch_from_remote nested loop and error handling

    #[tokio::test]
    async fn test_fetch_from_remote_multiple_issues_with_worklogs() {
        let now = Local::now();
        let worklog1 = create_local_entry(
            "l1",
            Some("p1"),
            now,
            Some(now),
            None,
            Some("ISSUE-1"),
            Some(now),
        );
        let worklog2 = create_local_entry(
            "l2",
            Some("p2"),
            now,
            Some(now),
            None,
            Some("ISSUE-1"),
            Some(now),
        );
        let worklog3 = create_local_entry(
            "l3",
            Some("p3"),
            now,
            Some(now),
            None,
            Some("ISSUE-2"),
            Some(now),
        );

        let sync_service = create_sync_service_with_mocks(
            |_| {},
            |issue_repo| {
                issue_repo
                    .expect_find_keys_with_worklogs()
                    .returning(|| Ok(vec!["ISSUE-1".to_string(), "ISSUE-2".to_string()]));
            },
            move |tracker| {
                tracker
                    .expect_get_worklogs_for_issue()
                    .with(eq("ISSUE-1"), always())
                    .returning(move |_, _| Ok(vec![worklog1.clone(), worklog2.clone()]));
                tracker
                    .expect_get_worklogs_for_issue()
                    .with(eq("ISSUE-2"), always())
                    .returning(move |_, _| Ok(vec![worklog3.clone()]));
            },
        );

        let result = sync_service.fetch_from_remote(Some(30)).await.unwrap();

        assert_eq!(result.len(), 3);
        assert!(result.iter().any(|w| w.provider_worklog_id == "p1"));
        assert!(result.iter().any(|w| w.provider_worklog_id == "p2"));
        assert!(result.iter().any(|w| w.provider_worklog_id == "p3"));
    }

    #[tokio::test]
    async fn test_fetch_from_remote_handles_fetch_errors() {
        let now = Local::now();
        let worklog1 = create_local_entry(
            "l1",
            Some("p1"),
            now,
            Some(now),
            None,
            Some("ISSUE-1"),
            Some(now),
        );

        let sync_service = create_sync_service_with_mocks(
            |_| {},
            |issue_repo| {
                issue_repo
                    .expect_find_keys_with_worklogs()
                    .returning(|| Ok(vec!["ISSUE-1".to_string(), "ISSUE-2".to_string()]));
            },
            move |tracker| {
                tracker
                    .expect_get_worklogs_for_issue()
                    .with(eq("ISSUE-1"), always())
                    .returning(move |_, _| Ok(vec![worklog1.clone()]));
                tracker
                    .expect_get_worklogs_for_issue()
                    .with(eq("ISSUE-2"), always())
                    .returning(|_, _| {
                        Err(WorklogError::IssueTrackerError("Network error".to_string()))
                    });
            },
        );

        let result = sync_service.fetch_from_remote(Some(30)).await.unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].provider_worklog_id, "p1");
    }

    #[tokio::test]
    async fn test_fetch_from_remote_skips_entries_without_provider_id() {
        let now = Local::now();
        let worklog1 = create_local_entry(
            "l1",
            Some("p1"),
            now,
            Some(now),
            None,
            Some("ISSUE-1"),
            Some(now),
        );
        let worklog2 =
            create_local_entry("l2", None, now, Some(now), None, Some("ISSUE-1"), Some(now));

        let sync_service = create_sync_service_with_mocks(
            |_| {},
            |issue_repo| {
                issue_repo
                    .expect_find_keys_with_worklogs()
                    .returning(|| Ok(vec!["ISSUE-1".to_string()]));
            },
            move |tracker| {
                tracker
                    .expect_get_worklogs_for_issue()
                    .returning(move |_, _| Ok(vec![worklog1.clone(), worklog2.clone()]));
            },
        );

        let result = sync_service.fetch_from_remote(Some(30)).await.unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].provider_worklog_id, "p1");
    }

    #[tokio::test]
    async fn test_fetch_from_remote_validates_stopped_at() {
        let now = Local::now();
        let worklog = create_local_entry(
            "l1",
            Some("p1"),
            now,
            Some(now),
            None,
            Some("ISSUE-1"),
            None,
        );

        let sync_service = create_sync_service_with_mocks(
            |_| {},
            |issue_repo| {
                issue_repo
                    .expect_find_keys_with_worklogs()
                    .returning(|| Ok(vec!["ISSUE-1".to_string()]));
            },
            move |tracker| {
                tracker
                    .expect_get_worklogs_for_issue()
                    .returning(move |_, _| Ok(vec![worklog.clone()]));
            },
        );

        let result = sync_service.fetch_from_remote(Some(30)).await;

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("has no stopped_at"));
    }

    #[tokio::test]
    async fn test_fetch_from_remote_validates_time_spent() {
        let now = Local::now();
        let mut worklog = create_local_entry(
            "l1",
            Some("p1"),
            now,
            Some(now),
            None,
            Some("ISSUE-1"),
            Some(now),
        );
        worklog.time_spent_seconds = None;

        let sync_service = create_sync_service_with_mocks(
            |_| {},
            |issue_repo| {
                issue_repo
                    .expect_find_keys_with_worklogs()
                    .returning(|| Ok(vec!["ISSUE-1".to_string()]));
            },
            move |tracker| {
                tracker
                    .expect_get_worklogs_for_issue()
                    .returning(move |_, _| Ok(vec![worklog.clone()]));
            },
        );

        let result = sync_service.fetch_from_remote(Some(30)).await;

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("has no time_spent_seconds"));
    }

    #[tokio::test]
    async fn test_fetch_from_remote_no_local_issues() {
        let sync_service = create_sync_service_with_mocks(
            |_| {},
            |issue_repo| {
                issue_repo
                    .expect_find_keys_with_worklogs()
                    .returning(|| Ok(vec![]));
            },
            |_| {},
        );

        let result = sync_service.fetch_from_remote(Some(30)).await.unwrap();

        assert_eq!(result.len(), 0);
    }

    #[tokio::test]
    async fn test_sync_handles_local_duplicates_safely() {
        // SAFE BEHAVIOR: When local has duplicates with same provider_worklog_id,
        // we warn about them and use the newest for sync comparison,
        // but DON'T delete them during sync (to avoid propagating to remote).

        let now = Local::now();
        let now_utc = Utc::now();

        // Create 3 local entries with same provider_worklog_id (duplicates!)
        // Make them slightly different in updated_at so we can tell which is newest
        let local1 = create_local_entry(
            "l1",
            Some("p1"),
            now - chrono::Duration::seconds(2),
            Some(now - chrono::Duration::seconds(2)),
            None,
            Some("ISSUE-1"),
            Some(now - chrono::Duration::seconds(2)),
        );
        let local2 = create_local_entry(
            "l2",
            Some("p1"),
            now - chrono::Duration::seconds(1),
            Some(now - chrono::Duration::seconds(1)),
            None,
            Some("ISSUE-1"),
            Some(now - chrono::Duration::seconds(1)),
        );
        let local3 = create_local_entry(
            "l3",
            Some("p1"),
            now, // Newest
            Some(now),
            None,
            Some("ISSUE-1"),
            Some(now),
        );

        // Remote has only 1 entry with provider_worklog_id "p1"
        let remote = create_remote_entry("p1", now_utc, "ISSUE-1");

        let local1_clone = local1.clone();
        let local2_clone = local2.clone();
        let local3_clone = local3.clone();

        let sync_service = create_sync_service_with_mocks(
            move |repo| {
                // Return all 3 duplicates when finding all
                let l1 = local1_clone.clone();
                let l2 = local2_clone.clone();
                let l3 = local3_clone.clone();
                repo.expect_find_all()
                    .returning(move || Ok(vec![l1.clone(), l2.clone(), l3.clone()]));

                // Should NOT delete duplicates during sync
                repo.expect_delete().times(0);

                repo.expect_find_by_provider_id().returning(|_| Ok(None));

                repo.expect_is_tombstoned().returning(|_| Ok(false));
                repo.expect_cleanup_tombstones().returning(|| Ok(0));
                repo.expect_update().returning(|_| Ok(()));
            },
            |issue_repo| {
                issue_repo
                    .expect_find_keys_with_worklogs()
                    .returning(|| Ok(vec!["ISSUE-1".to_string()]));
            },
            move |tracker| {
                let remote = remote.clone();
                tracker
                    .expect_get_worklogs_for_issue()
                    .returning(move |_, _| Ok(vec![remote.to_worklog_entry()]));
            },
        );

        // Run sync - should warn about duplicates but not delete them
        let result = sync_service.sync(Some(30)).await.unwrap();

        // Safe behavior: Duplicates are detected and warned about,
        // but not deleted (to avoid accidental remote deletion).
        // Only the newest (l3) is used for sync comparison.
        assert_eq!(result.conflicts, 0, "No conflicts");
    }

    #[tokio::test]
    async fn test_mass_duplicates_safe_handling() {
        // SAFE BEHAVIOR: When you have many worklogs with duplicates,
        // we warn about them but don't delete during sync (to avoid propagating to remote).
        // This test simulates having 10 entries (many duplicates) vs 3 in remote.

        let now = Local::now();
        let now_utc = Utc::now();

        // Simulate 10 local worklogs: 3 unique provider IDs, but with duplicates
        // p1: 4 duplicates (l1, l2, l3, l4) - l4 is newest
        // p2: 3 duplicates (l5, l6, l7) - l7 is newest
        // p3: 3 duplicates (l8, l9, l10) - l10 is newest
        let mut local_worklogs = vec![];

        // Provider ID "p1" - 4 duplicates (with increasing timestamps)
        for i in 1..=4 {
            local_worklogs.push(create_local_entry(
                &format!("l{i}"),
                Some("p1"),
                now + chrono::Duration::seconds(i64::from(i)),
                Some(now + chrono::Duration::seconds(i64::from(i))),
                None,
                Some("ISSUE-1"),
                Some(now + chrono::Duration::seconds(i64::from(i))),
            ));
        }

        // Provider ID "p2" - 3 duplicates
        for i in 5..=7 {
            local_worklogs.push(create_local_entry(
                &format!("l{i}"),
                Some("p2"),
                now + chrono::Duration::seconds(i64::from(i)),
                Some(now + chrono::Duration::seconds(i64::from(i))),
                None,
                Some("ISSUE-1"),
                Some(now + chrono::Duration::seconds(i64::from(i))),
            ));
        }

        // Provider ID "p3" - 3 duplicates
        for i in 8..=10 {
            local_worklogs.push(create_local_entry(
                &format!("l{i}"),
                Some("p3"),
                now + chrono::Duration::seconds(i64::from(i)),
                Some(now + chrono::Duration::seconds(i64::from(i))),
                None,
                Some("ISSUE-1"),
                Some(now + chrono::Duration::seconds(i64::from(i))),
            ));
        }

        // Remote has only 3 unique entries (one for each provider ID)
        let remote1 = create_remote_entry("p1", now_utc, "ISSUE-1");
        let remote2 = create_remote_entry("p2", now_utc, "ISSUE-1");
        let remote3 = create_remote_entry("p3", now_utc, "ISSUE-1");

        let locals_clone = local_worklogs.clone();

        let sync_service = create_sync_service_with_mocks(
            move |repo| {
                // Return all 10 duplicates
                let locals = locals_clone.clone();
                repo.expect_find_all().returning(move || Ok(locals.clone()));

                // Should NOT delete duplicates during sync
                repo.expect_delete().times(0);

                repo.expect_find_by_provider_id().returning(|_| Ok(None));

                repo.expect_is_tombstoned().returning(|_| Ok(false));
                repo.expect_cleanup_tombstones().returning(|| Ok(0));
                repo.expect_update().returning(|_| Ok(()));
            },
            |issue_repo| {
                issue_repo
                    .expect_find_keys_with_worklogs()
                    .returning(|| Ok(vec!["ISSUE-1".to_string()]));
            },
            move |tracker| {
                let r1 = remote1.clone();
                let r2 = remote2.clone();
                let r3 = remote3.clone();
                tracker
                    .expect_get_worklogs_for_issue()
                    .returning(move |_, _| {
                        Ok(vec![
                            r1.to_worklog_entry(),
                            r2.to_worklog_entry(),
                            r3.to_worklog_entry(),
                        ])
                    });
            },
        );

        let result = sync_service.sync(Some(30)).await.unwrap();

        // Safe behavior: Duplicates remain locally (not deleted during sync).
        // Only the newest of each provider_id is used for comparison.
        // This prevents accidental remote deletion.
        assert_eq!(result.conflicts, 0, "No conflicts");
    }
}
