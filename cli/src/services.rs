//! Direct service composition for CLI - bypasses Runtime
use std::sync::Arc;
use worklog_config::config;
use worklog_core::{
    IssueService, IssueTrackerClient, SyncService, UserService, WorklogError, WorklogService,
    WorklogStorage,
};
use worklog_jira_adapter::JiraAdapter;
use worklog_sqlite_storage::SqliteStorage;

pub(crate) struct Services {
    pub worklog: Arc<WorklogService>,
    pub issue: Arc<IssueService>,
    pub user: Arc<UserService>,
    pub sync: Arc<SyncService>,
}

impl Services {
    /// Create services by composing storage + issue tracker directly
    pub fn new() -> Result<Self, WorklogError> {
        let config = config::load_with_keychain_lookup()
            .map_err(|e| WorklogError::ConfigError(e.to_string()))?;

        let storage = SqliteStorage::from_url(&config.storage.url, Some(&config.storage.options))?;

        let issue_tracker: Arc<dyn IssueTrackerClient> = Arc::new(JiraAdapter::from_url(
            &config.issue_tracker.url,
            &config.issue_tracker.config,
        )?);

        let worklog_repo = storage.worklog_repository();
        let issue_repo = storage.issue_repository();
        let user_repo = storage.user_repository();

        let worklog = Arc::new(WorklogService::new(
            worklog_repo.clone(),
            issue_tracker.clone(),
        ));
        let issue = Arc::new(IssueService::new(issue_repo.clone(), issue_tracker.clone()));
        let user = Arc::new(UserService::new(user_repo.clone(), issue_tracker.clone()));
        let sync = Arc::new(SyncService::new(
            worklog_repo,
            issue_repo,
            issue_tracker.clone(),
        ));

        Ok(Self {
            worklog,
            issue,
            user,
            sync,
        })
    }
}

pub fn get_services() -> Result<Services, WorklogError> {
    Services::new()
}
