use crate::error::WorklogError;
use config::AppConfiguration;
use jira::models::issue::IssueSummary;
use jira::{Credentials, Jira};
use operation::{
    add::{self, Add},
    del::{self, Del},
    issues,
    sync::Sync,
};
use std::path::PathBuf;
use storage::dbms_repository::{DbmsRepository};
use types::LocalWorklog;

pub mod config;
pub mod date;
pub mod error;
pub mod operation;
pub mod storage;

pub mod types;

pub struct ApplicationRuntime {
    #[allow(dead_code)]
    config: AppConfiguration,
    client: Jira,
    worklog_service: DbmsRepository,
}

pub enum Operation {
    Add(Add),
    Del(Del),
    Codes,
    Sync(Sync),
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
        let config = config::load()?;

        let client = Jira::new(
            &config.jira.url,
            Credentials::Basic(config.jira.user.clone(), config.jira.token.clone()),
        )?;

        let path = PathBuf::from(&config.application_data.local_worklog);

        if !path.exists() {
            println!("No support for the old journal. Use 'timesheet sync' to get your work logs from Jira");
        }

        let worklog_service = DbmsRepository::new(&path)?;

        Ok(ApplicationRuntime {
            config,
            client,
            worklog_service,
        })
    }

    pub fn jira_client(&self) -> &Jira {
        &self.client
    }

    pub fn worklog_service(&self) -> &DbmsRepository {
        &self.worklog_service
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
                let issues = issues::execute(self).await?;
                Ok(OperationResult::IssueSummaries(issues))
            }
            Operation::Sync(sync_cmd) => {
                operation::sync::execute(self, &sync_cmd).await?;
                Ok(OperationResult::Synchronised)
            }
        }
    }
}
