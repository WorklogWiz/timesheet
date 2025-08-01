//! The `worklog` crate provides a comprehensive system for managing and tracking work logs
//! in integration with Jira. It offers functionalities for creating, reading, updating,
//! and deleting work logs both locally and on Jira servers.
//!
//! # Features
//!
//! - Local work log storage with `SQLite` database
//! - Jira integration for synchronizing work logs
//! - User management and authentication
//! - Issue tracking and component management
//! - Timer functionality for tracking work duration
//!
//! # Main Components
//!
//! - [`ApplicationRuntime`]: The main runtime environment coordinating all services
//! - [`WorkLogService`]: Handles work log management operations
//! - [`UserService`]: Manages user-related operations
//! - [`IssueService`]: Handles Jira issue operations
//! - [`ComponentService`]: Manages Jira components
//! - [`TimerService`]: Provides time tracking functionality
//!
//! # Example
//!
//! ```no_run
//! use worklog::ApplicationRuntimeBuilder;
//! use worklog::Operation;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!
//!     let runtime = ApplicationRuntimeBuilder::new()
//!         .build()?;
//!
//!     // Execute various operations
//!     let result = runtime.execute(Operation::Codes).await?;
//!
//!     Ok(())
//! }
//! ```
//!
//! The new example demonstrates how to use the `ApplicationRuntimeBuilder` with its fluent interface to customize the runtime with specific configuration options before building it. This builder pattern gives users more flexibility compared to the simple `ApplicationRuntime::new()` approach in the original example.

use crate::config::JiraClientConfiguration;
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
pub use crate::service::timer::TimerService;
use crate::service::user::UserService;
use crate::service::worklog::WorkLogService;
use config::AppConfiguration;
use jira::builder::JiraBuilder;
use jira::models::issue::IssueSummary;
use jira::{Credentials, Jira};
use log::debug;
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
    pub jira_client: Jira,
    pub worklog_service: Arc<WorkLogService>,
    pub user_service: Arc<UserService>,
    pub issue_service: Arc<IssueService>,
    pub component_service: Arc<ComponentService>,
    pub timer_service: Arc<TimerService>,
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
        &self.jira_client
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

    #[must_use]
    pub fn timer_service(&self) -> Arc<TimerService> {
        self.timer_service.clone()
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
/// ```rust,no_run
/// // Creates a runtime with a brand new database in memory with
/// // a jira instance according whatever is in the config file
/// use worklog::ApplicationRuntimeBuilder;
///
/// let runtime = ApplicationRuntimeBuilder::new()
///     .use_in_memory_db()
///     .use_jira_test_instance()
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
/// ```rust,no_run
/// use worklog::ApplicationRuntimeBuilder;
///
/// let runtime = ApplicationRuntimeBuilder::new()
///     .build()
///     .expect("Failed to create runtime");
/// ```
///
/// ## Creating a Runtime with In-Memory Database and Jira Test Instance
/// ```rust
/// use worklog::ApplicationRuntimeBuilder;
///
/// let runtime = ApplicationRuntimeBuilder::new()
///     .use_in_memory_db()
///     .use_jira_test_instance()
///     .build()
///     .expect("Failed to create runtime with test Jira instance");
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
    use_in_memory_db: bool,       // Internal field to toggle in-memory mode.
    use_jira_test_instance: bool, // Internal field to toggle Jira test instance.
}

impl Default for ApplicationRuntimeBuilder {
    fn default() -> Self {
        ApplicationRuntimeBuilder {
            use_in_memory_db: false,
            use_jira_test_instance: false,
            config: AppConfiguration {
                jira: JiraClientConfiguration {
                    url: "https://norns.atlassian.net".to_string(),
                    user: "<USER>".to_string(),
                    token: "<PASSWORD>".to_string(),
                },
                application_data: config::ApplicationData {
                    local_worklog: "local_worklog.db".to_string(),
                },
            },
        }
    }
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
    /// ```rust,no_run
    /// use worklog::ApplicationRuntimeBuilder;
    ///
    /// let builder = ApplicationRuntimeBuilder::new().build().expect("Failed to create app runtime");
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
        Self::default()
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
    /// ```rust
    /// use worklog::ApplicationRuntimeBuilder;
    ///
    /// let runtime = ApplicationRuntimeBuilder::new()
    ///     .use_in_memory_db()
    ///     .use_jira_test_instance()
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

    /// Configures the `ApplicationRuntime` to use a test Jira instance instead of
    /// the one specified in the configuration file.
    ///
    /// When this option is enabled, the runtime will use environment variables to configure
    /// the Jira client instead of the configuration file settings. This is particularly
    /// useful for testing scenarios where you want to use a mock or test Jira instance.
    ///
    /// # Returns
    ///
    /// Returns the updated `ApplicationRuntimeBuilder` instance.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use worklog::ApplicationRuntimeBuilder;
    ///
    /// let runtime = ApplicationRuntimeBuilder::new()
    ///     .use_jira_test_instance() // Use test Jira instance
    ///     .build()
    ///     .expect("Failed to create runtime with test Jira instance");
    /// ```
    ///
    /// The test instance will be configured using environment variables instead of
    /// the configuration file settings.
    ///
    /// See documentation for `JiraBuilder` for details about the environment variables.
    #[must_use]
    pub fn use_jira_test_instance(mut self) -> Self {
        self.use_jira_test_instance = true;
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
    /// ```rust
    /// use worklog::ApplicationRuntimeBuilder;
    ///
    /// let runtime = ApplicationRuntimeBuilder::new()
    ///     .use_in_memory_db() // Configure for in-memory database, useful for testing
    ///     .use_jira_test_instance()
    ///     .build()
    ///     .expect("Failed to build ApplicationRuntime");
    ///
    /// // Use the runtime for application operations
    /// ```
    ///
    ///
    /// # Errors
    ///
    /// - [`WorklogError::ConfigFileNotFound`]: This error occurs if the configuration fails to load during initialization or contains invalid values.
    /// - [`WorklogError::DatabaseError`]: This error occurs if the database connection manager fails to initialize either due to invalid configuration or runtime errors.
    /// - [`WorklogError::JiraBuildError`]: This error occurs if the Jira client fails to initialize, such as when provided with incorrect credentials or an invalid URL.
    pub fn build(&mut self) -> Result<ApplicationRuntime, WorklogError> {
        let jira_client = self.create_jira_client()?;

        let database_manager = &self.create_database_manager()?;

        let user_repo = database_manager.create_user_repository();
        let worklog_repo = database_manager.create_worklog_repository();
        let issue_repo = database_manager.create_issue_repository();
        let component_repo = database_manager.create_component_repository();
        let timer_repo = database_manager.create_timer_repository();

        let user_service = Arc::new(UserService::new(user_repo));
        let issue_service = Arc::new(IssueService::new(issue_repo));
        let worklog_service = Arc::new(WorkLogService::new(
            worklog_repo,
            issue_service.clone(),
            jira_client.clone(),
        ));
        let component_service = Arc::new(ComponentService::new(component_repo.clone()));
        let timer_service = Arc::new(TimerService::new(
            timer_repo,
            Arc::clone(&issue_service),
            Arc::clone(&worklog_service),
            jira_client.clone(),
        ));

        Ok(ApplicationRuntime {
            jira_client,
            worklog_service,
            user_service,
            issue_service,
            component_service,
            timer_service,
        })
    }

    /// Creates and configures a Jira client based on the builder's settings.
    ///
    /// Note! If `!use_jira_test_instance`, the disk configuration file will be loaded into
    /// the `config` field.
    ///
    /// This method handles two different scenarios for creating a Jira client:
    /// 1. Using environment variables (when `use_jira_test_instance` is true)
    /// 2. Using configuration file settings (when `use_jira_test_instance` is false)
    ///
    /// # Returns
    ///
    /// Returns a Result containing either:
    /// - `Ok(Jira)`: A configured Jira client instance
    /// - `Err(WorklogError)`: An error if client creation fails
    ///
    /// # Errors
    ///
    /// This method can return several types of errors:
    /// - `WorklogError::JiraBuildError`: When environment variables are missing or invalid
    /// - `WorklogError::ConfigFileNotFound`: When the configuration file cannot be loaded
    /// - `WorklogError::JiraError`: When the Jira client fails to initialize with provided credentials
    ///
    fn create_jira_client(&mut self) -> Result<Jira, WorklogError> {
        if self.use_jira_test_instance {
            // Use environment variables for test instance
            JiraBuilder::create_from_env().map_err(WorklogError::JiraBuildError)
        } else {
            // Load configuration from disk file to obtain Jira credentials
            self.config = config::load_with_keychain_lookup()?;
            self.create_jira_from_config()
        }
    }

    /// Helper method to create a Jira client from the current configuration
    fn create_jira_from_config(&self) -> Result<Jira, WorklogError> {
        let credentials = Credentials::Basic(
            self.config.jira.user.clone(),
            self.config.jira.token.clone(),
        );

        Jira::new(&self.config.jira.url, credentials)
            .map_err(|e| WorklogError::JiraError(e.to_string()))
    }

    fn create_database_manager(&self) -> Result<DatabaseManager, WorklogError> {
        // If we are running in memory, we have no need for the optionally loaded config.
        let database_manager = if self.use_in_memory_db {
            debug!("Using in-memory database");
            DatabaseManager::new(&DatabaseConfig::SqliteInMemory)?
        } else {
            debug!(
                "Opening database located in {}",
                self.config.application_data.local_worklog
            );

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
