use std::collections::HashMap;
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
    #[allow(dead_code)]
    pub user: Arc<UserService>,
    pub sync: Arc<SyncService>,
}

impl Services {
    /// Create services with Jira support (requires config file)
    pub fn new() -> Result<Self, WorklogError> {
        let config = config::load_with_keychain_lookup()
            .map_err(|e| WorklogError::ConfigError(e.to_string()))?;

        let issue_tracker: Arc<dyn IssueTrackerClient> = Arc::new(JiraAdapter::from_url(
            &config.issue_tracker.url,
            &config.issue_tracker.config,
        )?);

        let storage_url = format!("sqlite://{}", config::worklog_file().display());
        let storage = SqliteStorage::from_url(&storage_url, None)?;

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

    /// Create services in local-only mode (no Jira)
    pub fn new_local_only() -> Result<Self, WorklogError> {
        let db_path = config::worklog_file();

        // Ensure parent directory exists
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                WorklogError::ConfigError(format!("Failed to create data directory: {e}"))
            })?;
        }

        let storage_url = format!("sqlite://{}", db_path.display());
        let storage = SqliteStorage::from_url(&storage_url, None)?;

        let mut jira_config = HashMap::new();
        jira_config.insert("username".to_string(), "local".to_string());
        jira_config.insert("token".to_string(), "local".to_string());

        let issue_tracker: Arc<dyn IssueTrackerClient> =
            Arc::new(JiraAdapter::from_url("jira://localhost", &jira_config)?);

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
