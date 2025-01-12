use crate::error::WorklogError;
use crate::repository::database_manager::{DatabaseConfig, DatabaseManager};
use crate::repository::sqlite::sqlite_component_repo::SqliteComponentRepository;
use crate::repository::sqlite::sqlite_issue_repo::SqliteIssueRepository;
use crate::repository::sqlite::sqlite_user_repo::SqliteUserRepository;
use crate::repository::sqlite::sqlite_worklog_repo::SqliteWorklogRepository;
use crate::service::component_service::ComponentService;
use crate::service::issue_service::IssueService;
use crate::service::user_service::UserService;
use crate::service::worklog_service::WorkLogService;
use config::AppConfiguration;
use jira::models::issue::IssueSummary;
use jira::{Credentials, Jira};
use operation::{
    add::{self, Add},
    codes,
    del::{self, Del},
};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use types::LocalWorklog;

pub mod config;
pub mod date;
pub mod error;
pub mod operation;

pub mod types;

pub mod repository;
pub mod service;

pub struct ApplicationRuntime {
    #[allow(dead_code)]
    config: AppConfiguration,
    client: Jira,
    pub worklog_service: Arc<WorkLogService<SqliteWorklogRepository>>,
    pub user_service: Arc<UserService<SqliteUserRepository>>,
    pub issue_service: Arc<IssueService<SqliteIssueRepository>>,
    pub component_service: Arc<ComponentService<SqliteComponentRepository>>,
}

pub enum Operation {
    Add(Add),
    Del(Del),
    Codes,
    Sync(operation::sync::Sync),
}

pub enum OperationResult {
    Added(Vec<LocalWorklog>),
    Deleted(String),
    IssueSummaries(Vec<IssueSummary>),
    Synchronised,
}

impl ApplicationRuntime {
    /// Creates a new instance of `ApplicationRuntime`.
    ///
    /// # Returns
    ///
    /// If successful, returns an `ApplicationRuntime` instance configured with the application
    /// settings and services like Jira client and worklog storage. Returns an error of type
    /// `WorklogError` if there's an issue during initialization.
    ///
    /// # Errors
    ///
    /// - Returns an error if the configuration fails to load.
    /// - Returns an error if the creation of the Jira client fails.
    /// - Returns an error if the initialization of the local worklog storage fails.
    ///
    /// # Notes
    ///
    /// - If the local worklog path does not exist, a warning is printed, and syncing with Jira is recommended.
    pub fn new() -> Result<Self, WorklogError> {
        ApplicationRuntimeBuilder::new().build()
    }

    pub fn jira_client(&self) -> &Jira {
        &self.client
    }

    pub fn worklog_service(&self) -> Arc<WorkLogService<SqliteWorklogRepository>> {
        self.worklog_service.clone()
    }

    pub fn user_service(&self) -> Arc<UserService<SqliteUserRepository>> {
        self.user_service.clone()
    }

    pub fn issue_service(&self) -> Arc<IssueService<SqliteIssueRepository>> {
        self.issue_service.clone()
    }

    pub fn component_service(&self) -> Arc<ComponentService<SqliteComponentRepository>> {
        self.component_service.clone()
    }

    /// Executes the specified `Operation` and returns the result.
    ///
    /// # Arguments
    ///
    /// * `operation` - The operation to be executed.
    ///
    /// # Returns
    ///
    /// A `Result` wrapping an `OperationResult` on success, or a `WorklogError` on failure.
    ///
    /// # Errors
    ///
    /// This function may return an error (`WorklogError`) in the following scenarios:
    ///
    /// - When adding worklogs fails during `Operation::Add`.
    /// - When deleting a worklog entry fails during `Operation::Del`.
    /// - When fetching issue summaries fails during `Operation::Codes`.
    /// - When syncing worklogs with Jira fails during `Operation::Sync`.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use your_crate::ApplicationRuntime;
    /// use your_crate::operation::Operation;
    ///
    /// async fn example(runtime: &ApplicationRuntime) {
    ///     let operation = Operation::Sync(Sync::new());
    ///     match runtime.execute(operation).await {
    ///         Ok(result) => println!("Operation successful: {:?}", result),
    ///         Err(err) => eprintln!("Operation failed: {:?}", err),
    ///     }
    /// }
    /// ```
    pub async fn execute(&self, operation: Operation) -> Result<OperationResult, WorklogError> {
        match operation {
            Operation::Add(mut instructions) => {
                let worklogs = add::execute(self, &mut instructions).await?;
                Ok(OperationResult::Added(worklogs))
            }
            Operation::Del(instructions) => {
                let id = del::execute(self, &instructions).await?;
                Ok(OperationResult::Deleted(id))
            }
            Operation::Codes => {
                let issues = codes::execute(self).await?;
                Ok(OperationResult::IssueSummaries(issues))
            }
            Operation::Sync(sync_cmd) => {
                operation::sync::execute(self, &sync_cmd).await?;
                Ok(OperationResult::Synchronised)
            }
        }
    }
}

pub struct ApplicationRuntimeBuilder {
    config: AppConfiguration,
    use_in_memory_db: bool, // Internal field to toggle in-memory mode.
}

impl ApplicationRuntimeBuilder {
    pub fn new() -> Self {
        // Load the configuration from disk as the default.
        let config = config::load().expect("Failed to load configuration");
        Self {
            config,
            use_in_memory_db: false,
        }
    }

    /// Override to use an in-memory SQLite database, used for testing.
    pub fn use_in_memory_db(mut self) -> Self {
        self.use_in_memory_db = true;
        self
    }

    /// Builds the runtime, applying any overrides dynamically.
    pub fn build(self) -> Result<ApplicationRuntime, WorklogError> {
        let database_manager = if self.use_in_memory_db {
            DatabaseManager::new(&DatabaseConfig::SqliteInMemory)?
        } else {
            let path = PathBuf::from(&self.config.application_data.local_worklog);
            if let Some(parent) = path.parent() {
                if !parent.exists() {
                    fs::create_dir_all(parent)?;
                }
            }

            DatabaseManager::new(&DatabaseConfig::SqliteOnDisk { path })?
        };

        let client = Jira::new(
            &self.config.jira.url,
            Credentials::Basic(
                self.config.jira.user.clone(),
                self.config.jira.token.clone(),
            ),
        )?;

        let user_repo = database_manager.create_user_repository();
        let worklog_repo = database_manager.create_worklog_repository();
        let issue_repo = database_manager.create_issue_repository();
        let component_repo = database_manager.create_component_repository();

        let user_service = Arc::new(UserService::new(user_repo));
        let worklog_service = Arc::new(WorkLogService::new(worklog_repo));
        let issue_service = Arc::new(IssueService::new(issue_repo));
        let component_service = Arc::new(ComponentService::new(component_repo));

        Ok(ApplicationRuntime {
            config: self.config,
            client,
            worklog_service,
            user_service,
            issue_service,
            component_service,
        })
    }
}
