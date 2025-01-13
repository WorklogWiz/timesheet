/// The `ApplicationRuntime` struct serves as the main runtime environment for the application,
/// providing access to essential services such as issue management, user management, and
/// worklog management. It facilitates communication with the Jira API and local worklog
/// storage.
///
/// # Fields
///
/// * `config` - Represents the application configuration, including settings for Jira and local
///   worklogs.
/// * `client` - An instance of the Jira client used to interact with the Jira API.
/// * `worklog_service` - A shared instance of the `WorkLogService` for managing worklogs.
/// * `user_service` - A shared instance of the `UserService` for managing Jira users.
/// * `issue_service` - A shared instance of the `IssueService` for managing issues.
/// * `component_service` - A shared instance of the `ComponentService` for managing components.
use crate::error::WorklogError;
use crate::repository::database_manager::{DatabaseConfig, DatabaseManager};
use crate::service::component::ComponentService;
use crate::service::issue::IssueService;
use crate::service::user::UserService;
use crate::service::worklog::WorkLogService;
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

pub(crate) mod repository;
pub mod service;

/// The `ApplicationRuntime` struct serves as the main runtime environment for the application,
/// providing access to essential services like issue management, user management, and
/// worklog management. It facilitates communication with the Jira API and local worklog
/// storage.
///
/// # Fields
///
/// * `client` - An instance of the Jira client used to interact with the Jira API.
/// * `worklog_service` - A shared instance of the `WorkLogService` for managing worklogs.
/// * `user_service` - A shared instance of the `UserService` for managing Jira users.
/// * `issue_service` - A shared instance of the `IssueService` for managing issues.
/// * `component_service` - A shared instance of the `ComponentService` for managing components.
///
/// # Notes
///
/// This struct plays a central role in orchestrating the different services and allowing
/// them to operate in harmony within the application runtime environment.
pub struct ApplicationRuntime {
    client: Jira,
    pub worklog_service: Arc<WorkLogService>,
    pub user_service: Arc<UserService>,
    pub issue_service: Arc<IssueService>,
    pub component_service: Arc<ComponentService>,
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

    #[must_use]
    pub fn jira_client(&self) -> &Jira {
        &self.client
    }

    #[must_use]
    pub fn worklog_service(&self) -> Arc<WorkLogService> {
        self.worklog_service.clone()
    }

    #[must_use]
    pub fn user_service(&self) -> Arc<UserService> {
        self.user_service.clone()
    }

    #[must_use]
    pub fn issue_service(&self) -> Arc<IssueService> {
        self.issue_service.clone()
    }

    #[must_use]
    pub fn component_service(&self) -> Arc<ComponentService> {
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

///
/// A builder for creating an instance of `ApplicationRuntime`.
///
/// The `ApplicationRuntimeBuilder` facilitates the configuration and creation of an
/// `ApplicationRuntime` instance with the ability to control specific behaviors like
/// using an in-memory database for testing purposes.
///
/// # Usage
///
/// The typical usage involves creating the builder via [`ApplicationRuntimeBuilder::new`],
/// optionally configuring it using its methods, and then calling [`ApplicationRuntimeBuilder::build`]
/// to produce an `ApplicationRuntime` instance.
///
/// ```rust,ignore
/// let runtime = ApplicationRuntimeBuilder::new()
///     .use_in_memory_db()
///     .build()
///     .expect("Failed to create runtime");
/// ```
///
/// # Builder Methods
///
/// - [`ApplicationRuntimeBuilder::new`] initializes the builder with default configuration.
/// - [`ApplicationRuntimeBuilder::use_in_memory_db`] configures the runtime to use an in-memory ``SQLite`` database,
///   suitable for testing.
/// - [`ApplicationRuntimeBuilder::build`] finalizes the configuration and creates the `ApplicationRuntime` instance.
///
/// # Examples
///
/// ## Creating a Runtime with Defaults
/// ```rust,ignore
/// let runtime = ApplicationRuntimeBuilder::new()
///     .build()
///     .expect("Failed to create runtime");
/// ```
///
/// ## Creating a Runtime with In-Memory Database
/// ```rust,ignore
/// let runtime = ApplicationRuntimeBuilder::new()
///     .use_in_memory_db()
///     .build()
///     .expect("Failed to create runtime");
/// ```
///
/// # Errors
///
/// [`ApplicationRuntimeBuilder::build`] may return a `WorklogError` if any of the necessary
/// components fail to initialize, such as the Jira client or database connection manager.
///
/// [`ApplicationRuntimeBuilder::new`]: #method.new
/// [`ApplicationRuntimeBuilder::use_in_memory_db`]: #method.use_in_memory_db
/// [`ApplicationRuntimeBuilder::build`]: #method.build
pub struct ApplicationRuntimeBuilder {
    config: AppConfiguration,
    use_in_memory_db: bool, // Internal field to toggle in-memory mode.
}

impl ApplicationRuntimeBuilder {
    /// Creates a new instance of `ApplicationRuntimeBuilder` with default settings.
    ///
    /// This method initializes the builder with the application's configuration loaded
    /// from disk. By default, the created builder uses an on-disk database for persistent
    /// storage. Use [`ApplicationRuntimeBuilder::use_in_memory_db`] to configure it for
    /// testing scenarios or temporary setups that do not require persistent storage.
    ///
    /// # Returns
    ///
    /// A new `ApplicationRuntimeBuilder` instance configured with default values.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use your_crate::ApplicationRuntimeBuilder;
    ///
    /// let builder = ApplicationRuntimeBuilder::new();
    /// let runtime = builder.build().expect("Failed to build ApplicationRuntime");
    /// ```
    ///
    /// # Errors
    ///
    /// It will panic if the application configuration cannot be loaded from disk.
    ///
    ///
    /// # Panics
    ///
    /// This method will panic if the application configuration cannot be loaded from disk.
    /// Ensure that the configuration file exists and is accessible before calling this method.
    #[must_use]
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        // Load the configuration from disk as the default.
        let config = config::load().expect("Failed to load configuration");
        Self {
            config,
            use_in_memory_db: false,
        }
    }

    
    /// Configures the `ApplicationRuntime` to use an in-memory database.
    ///
    /// # Returns
    ///
    /// Returns the updated `ApplicationRuntimeBuilder` instance.
    ///
    /// # Examples
    ///
    /// In the example below, we configure the runtime to use an in-memory database, 
    /// which is particularly useful for testing scenarios.
    ///
    /// ```rust,ignore
    /// let runtime = ApplicationRuntimeBuilder::new()
    ///     .use_in_memory_db()
    ///     .build()
    ///     .expect("Failed to create runtime with in-memory database");
    /// ```
    ///
    /// This setting prevents any changes to the local filesystem, avoiding persistent
    /// storage, and instead everything operates within memory.
    #[must_use]
    pub fn use_in_memory_db(mut self) -> Self {
        self.use_in_memory_db = true;
        self
    }

    
    /// Finalizes the construction of the `ApplicationRuntime` instance.
    ///
    /// This method initializes various components required by `ApplicationRuntime`, such as
    /// database connection manager, Jira client, and various repositories and services.
    ///
    /// # Returns
    ///
    /// - `Ok(ApplicationRuntime)` if the runtime is successfully created with all its components initialized.
    /// - `Err(WorklogError)` if initialization fails at any stage, such as when the database manager
    ///   or Jira client cannot be created.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use your_crate::ApplicationRuntimeBuilder;
    ///
    /// let runtime = ApplicationRuntimeBuilder::new()
    ///     .use_in_memory_db() // Configure for in-memory database, useful for testing
    ///     .build()
    ///     .expect("Failed to build ApplicationRuntime");
    ///
    /// // Use the runtime for application operations
    /// ```
    ///
    ///
    /// # Errors
    ///
    /// - [`WorklogError::ConfigurationError`]: This error occurs if the configuration fails to load during initialization or contains invalid values.
    /// - [`WorklogError::DatabaseError`]: This error occurs if the database connection manager fails to initialize either due to invalid configuration or runtime errors.
    /// - [`WorklogError::ClientInitializationError`]: This error occurs if the Jira client fails to initialize, such as when provided with incorrect credentials or an invalid URL.
    /// - [`WorklogError::IoError`]: This error occurs when there are issues with file system operations, such as failing to create required directories for on-disk databases.
    pub fn build(&self) -> Result<ApplicationRuntime, WorklogError> {
        let database_manager = &self.initialise_database_connection_manager()?;

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
        // assert_send_sync(&component_service);

        Ok(ApplicationRuntime {
            client,
            worklog_service,
            user_service,
            issue_service,
            component_service,
        })
    }

    fn initialise_database_connection_manager(&self) -> Result<DatabaseManager, WorklogError> {
        let database_manager = if self.use_in_memory_db {
            DatabaseManager::new(&DatabaseConfig::SqliteInMemory)?
        } else {
            let path = PathBuf::from(self.config.application_data.local_worklog.clone());
            if let Some(parent) = path.parent() {
                if !parent.exists() {
                    fs::create_dir_all(parent)?;
                }
            }

            DatabaseManager::new(&DatabaseConfig::SqliteOnDisk { path })?
        };
        Ok(database_manager)
    }
}

#[allow(dead_code)]
fn assert_send_sync<T: Send + Sync>(_: T) {}

#[cfg(test)]
mod tests {
    use super::*;

    /// Ensures that the `ApplicationRuntime` instance created using the builder
    /// is properly configured for concurrent usage and can support threading
    /// by implementing the `Send` and `Sync` traits.
    ///
    /// This test creates an in-memory runtime for testing purposes,
    /// avoiding file I/O while maintaining logical integrity of the runtime's services.
    ///
    /// # Usage
    ///
    /// Run the test using:
    ///
    /// ```bash
    /// cargo test test_create_in_memory_runtime
    /// ```
    ///
    /// # Assertions
    ///
    /// - The `ApplicationRuntime` instance must successfully initialize.
    /// - The runtime instance must implement `Send` and `Sync` traits.
    ///
    /// # Errors
    ///
    /// If the configuration cannot be loaded or any of the runtime's dependencies
    /// fail to initialize, the test will panic.
    #[test]
    pub fn test_create_in_memory_runtime() {
        let runtime = ApplicationRuntimeBuilder::new()
            .use_in_memory_db()
            .build()
            .unwrap();
        assert_send_sync(runtime);
    }
}
