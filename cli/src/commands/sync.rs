use chrono::{DateTime, Local};
use log::info;
use std::process::exit;
use worklog_core::WorklogError;

use crate::services::Services;

pub struct SyncOptions {
    pub started: Option<DateTime<Local>>,
    pub all_users: bool,
    pub projects: Vec<String>,
    pub issues: Vec<String>,
}

/// Execute sync command - performs three-way sync using `SyncService`
pub async fn execute(services: &Services, options: SyncOptions) -> Result<(), WorklogError> {
    // Ensure current user is fetched and cached
    services.user.ensure_current_user().await?;

    let days_back = options.started.map(|start_date| {
        let now = Local::now();
        let duration = now.signed_duration_since(start_date);
        duration.num_days()
    });

    if options.all_users || !options.projects.is_empty() || !options.issues.is_empty() {
        eprintln!("Warning: Filtering by users/projects/issues not yet supported");
        eprintln!("Syncing all recent worklogs for issues with local entries");
    }

    info!("Starting three-way synchronization...");
    let result = services.sync.sync(days_back).await?;

    // 5. Display results
    if result.added > 0 {
        info!("✓ Added {} worklogs from remote", result.added);
    }
    if result.updated > 0 {
        info!("✓ Updated {} worklogs from remote", result.updated);
    }
    if result.uploaded > 0 {
        info!("✓ Uploaded {} worklogs to remote", result.uploaded);
    }
    if result.deleted_local > 0 {
        info!("✓ Deleted {} worklogs locally", result.deleted_local);
    }
    if result.deleted_remote > 0 {
        info!("✓ Deleted {} worklogs from remote", result.deleted_remote);
    }

    if result.conflicts > 0 {
        eprintln!(
            "⚠️  {} conflicts detected - manual resolution required",
            result.conflicts
        );
        exit(2);
    }

    let total_changes = result.added
        + result.updated
        + result.uploaded
        + result.deleted_local
        + result.deleted_remote;
    if total_changes == 0 {
        info!("✓ Everything is in sync");
    } else {
        info!("✓ Sync complete: {total_changes} changes applied");
    }

    Ok(())
}
