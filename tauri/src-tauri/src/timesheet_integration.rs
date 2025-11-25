use crate::models::WorkSession;
use crate::services::Services;
use anyhow::Result;
use chrono::{DateTime, Local, Utc};
use std::sync::Arc;

/// Integration with the ~/timesheet worklog library
pub struct TimesheetIntegration {
    services: Arc<Services>,
    local_only_mode: bool,
}

impl TimesheetIntegration {
    /// Creates a new integration instance with Jira support (requires config file)
    pub fn new() -> Result<Self> {
        let services = Services::new()?;

        Ok(Self {
            services: Arc::new(services),
            local_only_mode: false,
        })
    }

    /// Creates a new integration instance in local-only mode (no Jira, no config needed)
    pub fn new_local_only() -> Result<Self> {
        let services = Services::new_local_only()?;

        Ok(Self {
            services: Arc::new(services),
            local_only_mode: true,
        })
    }

    /// Start a timer for an issue (uses worklog library)
    /// If `issue_key` is None, starts a timer without an issue (won't sync to Jira)
    pub async fn start_timer(
        &self,
        issue_key: Option<&str>,
        comment: Option<String>,
    ) -> Result<()> {
        // Start timer using new unified service
        self.services
            .worklog
            .start_timer(issue_key.map(String::from), Local::now(), comment)
            .await?;

        Ok(())
    }

    /// Stop the active timer and create a worklog entry
    pub async fn stop_timer(&self, comment: Option<String>) -> Option<WorkSession> {
        self.stop_timer_at(None, comment).await
    }

    /// Stop the active timer at a specific time
    pub async fn stop_timer_at(
        &self,
        stopped_at: Option<DateTime<Utc>>,
        comment: Option<String>,
    ) -> Option<WorkSession> {
        let stop_time = stopped_at.map_or_else(Local::now, |utc| utc.with_timezone(&Local));

        match self
            .services
            .worklog
            .stop_active_timer(stop_time, comment.clone())
            .await
        {
            Ok(entry) => {
                // Convert WorklogEntry to TaskSession
                let issue_key = if entry.issue_key.as_deref() == Some("UNASSIGNED")
                    || entry.issue_key.is_none()
                {
                    None
                } else {
                    entry.issue_key.clone()
                };

                let session = WorkSession {
                    id: entry.id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
                    issue_key,
                    start_date: entry.started_at.with_timezone(&chrono::Utc),
                    end_date: entry.stopped_at.map(|dt| dt.with_timezone(&chrono::Utc)),
                    comment: entry.comment,
                    in_progress: false,
                    synced_to_tracker: entry.synced_to_provider,
                    tracker_worklog_id: entry.provider_worklog_id.clone(),
                    time_codes: vec![], // Time codes are managed separately
                };
                Some(session)
            }
            Err(e) => {
                eprintln!("Error stopping timer: {e}");
                None
            }
        }
    }

    /// Get the currently active timer
    pub async fn get_active_timer(&self) -> Result<Option<WorkSession>> {
        match self.services.worklog.get_active_timer().await? {
            Some(entry) => {
                let issue_key = if entry.issue_key.as_deref() == Some("UNASSIGNED")
                    || entry.issue_key.is_none()
                {
                    None
                } else {
                    entry.issue_key.clone()
                };

                let session = WorkSession {
                    id: entry.id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
                    issue_key,
                    start_date: entry.started_at.with_timezone(&chrono::Utc),
                    end_date: entry.stopped_at.map(|dt| dt.with_timezone(&chrono::Utc)),
                    comment: entry.comment,
                    in_progress: true,
                    synced_to_tracker: entry.synced_to_provider,
                    tracker_worklog_id: entry.provider_worklog_id.clone(),
                    time_codes: vec![],
                };
                Ok(Some(session))
            }
            None => Ok(None),
        }
    }

    /// Sync completed timers to Jira (3-way sync)
    pub async fn sync_to_jira(&self) -> Result<usize> {
        let result = self.services.sync.sync(None).await?;
        Ok(result.uploaded + result.added + result.updated)
    }

    /// Full three-way sync using the new `SyncService`
    ///
    /// This performs fetch → compare → auto-merge, providing Git-like sync:
    /// - Downloads new remote entries
    /// - Uploads new local entries
    /// - Auto-merges non-conflicting changes
    /// - Detects and reports conflicts
    ///
    /// # Returns
    /// `SyncResult` with statistics and any detected conflicts
    pub async fn sync(&self, days_back: Option<i64>) -> Result<worklog_core::SyncResult> {
        if self.local_only_mode {
            return Ok(worklog_core::SyncResult::new());
        }

        self.services.sync.sync(days_back).await.map_err(Into::into)
    }

    /// Sync worklogs from Jira to local storage (download Jira → local)
    ///
    /// **DEPRECATED**: Use `sync()` instead for bidirectional sync
    pub async fn sync_from_jira(&self, days_back: Option<i64>) -> Result<usize> {
        use chrono::{Duration, Utc};

        if self.local_only_mode {
            return Ok(0);
        }

        // Default to 30 days back
        let days = days_back.unwrap_or(30);
        let start_date = Utc::now() - Duration::days(days);

        // Get issues that have local worklogs
        let issue_keys = self.services.issue.find_keys_with_worklogs().await?;

        if issue_keys.is_empty() {
            eprintln!("No issues to sync from Jira");
            return Ok(0);
        }

        eprintln!(
            "Syncing worklogs from Jira for {} issues (last {days} days)",
            issue_keys.len()
        );

        let mut total_synced = 0;

        for issue_key in &issue_keys {
            // Fetch worklogs for this issue from Jira
            match self
                .services
                .worklog
                .get_worklogs_for_issue(issue_key, start_date)
                .await
            {
                Ok(worklogs) => {
                    eprintln!("  {issue_key} - {} worklogs from Jira", worklogs.len());

                    for worklog in worklogs {
                        // Check if this worklog already exists locally
                        if let Some(provider_id) = &worklog.provider_worklog_id {
                            // Check by provider_worklog_id
                            let existing = self
                                .services
                                .worklog
                                .find_all()
                                .await?
                                .iter()
                                .any(|e| e.provider_worklog_id.as_ref() == Some(provider_id));

                            if existing {
                                continue; // Skip duplicates
                            }
                        }

                        // Add to local storage, mark as already synced
                        let mut entry = worklog;
                        entry.synced_to_provider = true;

                        self.services.worklog.add_entry(&entry).await?;
                        total_synced += 1;
                    }
                }
                Err(e) => {
                    eprintln!("  {issue_key} - Failed to fetch worklogs: {e}");
                }
            }
        }

        eprintln!("✓ Synced {total_synced} worklogs from Jira to local storage");
        Ok(total_synced)
    }

    /// Get all timers (completed and active) from the worklog library
    pub async fn get_all_timers(&self) -> Result<Vec<WorkSession>> {
        use chrono::Utc;

        // Get timers from the last 90 days
        let ninety_days_ago = Utc::now() - chrono::Duration::days(90);

        // Get all worklogs from the unified service
        let entries = self.services.worklog.find_after(ninety_days_ago).await?;

        // Convert WorklogEntry objects to TaskSession format
        let sessions: Vec<WorkSession> = entries
            .into_iter()
            .map(|entry| {
                let issue_key = if entry.issue_key.as_deref() == Some("UNASSIGNED")
                    || entry.issue_key.is_none()
                {
                    None
                } else {
                    entry.issue_key.clone()
                };

                WorkSession {
                    id: entry.id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
                    issue_key,
                    start_date: entry.started_at.with_timezone(&chrono::Utc),
                    end_date: entry.stopped_at.map(|dt| dt.with_timezone(&chrono::Utc)),
                    comment: entry.comment,
                    in_progress: entry.stopped_at.is_none(),
                    synced_to_tracker: entry.synced_to_provider,
                    tracker_worklog_id: entry.provider_worklog_id.clone(),
                    time_codes: vec![], // Time codes not used in worklog library
                }
            })
            .collect();

        Ok(sessions)
    }

    /// Assign an issue key to a session (timer/worklog entry)
    pub async fn assign_issue_to_session(&self, session_id: &str, issue_key: &str) -> Result<()> {
        // Find the worklog entry by ID
        let mut entry = self
            .services
            .worklog
            .find_by_id(session_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Session not found"))?;

        // Update the issue key
        entry.set_issue_key(Some(issue_key.to_string()));

        // Save the updated entry
        self.services.worklog.update_entry(&entry).await?;

        Ok(())
    }

    /// Get all Jira issues - combines local cache with fresh issues from Jira
    pub async fn get_all_issues(&self) -> Result<Vec<JiraIssue>> {
        use std::collections::HashSet;

        // Get all cached issues using new IssueService
        let local_issues = self.services.issue.get_all_issues().await?;

        println!("Found {} issues in local cache", local_issues.len());

        // Convert local issues to our model, filtering out UNASSIGNED placeholder
        let mut jira_issues: Vec<JiraIssue> = local_issues
            .into_iter()
            .filter(|issue| issue.key != "UNASSIGNED")
            .map(|issue| JiraIssue {
                key: issue.key,
                summary: issue.summary,
                from_cache: true,
            })
            .collect();

        // Keep track of keys we already have
        let mut seen_keys: HashSet<String> = jira_issues.iter().map(|i| i.key.clone()).collect();

        // Fetch recent issues from Jira (if not in local-only mode)
        if self.local_only_mode {
            println!("In local-only mode, skipping Jira fetch");
            println!("To see Jira issues, please configure Jira in Settings");
        } else {
            println!("Fetching fresh issues from Jira...");

            // Get issues the current user has recently worked on (via tracker)
            match self.services.issue.get_recently_worked_issues().await {
                Ok(fresh_issues) => {
                    println!("Fetched {} issues from Jira", fresh_issues.len());

                    // Filter for new issues not in cache
                    let new_issues: Vec<_> = fresh_issues
                        .into_iter()
                        .filter(|issue| !seen_keys.contains(&issue.key))
                        .collect();

                    if !new_issues.is_empty() {
                        println!("Found {} new issues to cache", new_issues.len());

                        // Save new issues to cache using the IssueService
                        self.services.issue.add_issues(&new_issues).await?;

                        // Add to result set
                        for issue in &new_issues {
                            jira_issues.push(JiraIssue {
                                key: issue.key.clone(),
                                summary: issue.summary.clone(),
                                from_cache: false,
                            });
                            seen_keys.insert(issue.key.clone());
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Failed to fetch fresh issues from Jira: {e}");
                    // Continue with just local cache
                }
            }
        }

        println!("Returning {} total issues", jira_issues.len());
        Ok(jira_issues)
    }

    /// Check if running in local-only mode
    pub fn is_local_only(&self) -> bool {
        self.local_only_mode
    }

    /// Add a new worklog entry
    pub async fn add_worklog_entry(&self, entry: &worklog_core::WorklogEntry) -> Result<String> {
        self.services
            .worklog
            .add_entry(entry)
            .await
            .map_err(Into::into)
    }

    /// Find a worklog entry by ID
    pub async fn find_worklog_by_id(&self, id: &str) -> Result<Option<worklog_core::WorklogEntry>> {
        self.services
            .worklog
            .find_by_id(id)
            .await
            .map_err(Into::into)
    }

    /// Update a worklog entry
    pub async fn update_worklog_entry(&self, entry: &worklog_core::WorklogEntry) -> Result<()> {
        self.services
            .worklog
            .update_entry(entry)
            .await
            .map_err(Into::into)
    }

    /// Delete a worklog entry
    pub async fn delete_worklog(&self, id: &str) -> Result<()> {
        self.services.worklog.delete(id).await.map_err(Into::into)
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct JiraIssue {
    pub key: String,
    pub summary: String,
    pub from_cache: bool,
}
