use crate::error::WorklogError;
use config::AppConfiguration;
use jira::{
    models::{core::JiraKey, issue::Issue},
    Jira,
};
use operation::{
    add::{self, Add},
    del::{self, Del},
    issues,
};
use std::path::PathBuf;
use storage::{LocalWorklog, WorklogStorage};

pub mod config;
pub mod date;
pub mod error;
pub mod operation;
pub mod storage;

pub struct ApplicationRuntime {
    config: AppConfiguration,
    client: Jira,
    worklog_service: WorklogStorage,
}

pub enum Operation {
    Add(Add),
    Del(Del),
    Codes,
}

pub enum OperationResult {
    Added(Vec<LocalWorklog>),
    Deleted(String),
    Issues(Vec<Issue>),
}

impl ApplicationRuntime {
    ///
    /// # Errors
    /// Returns an error if the initialisation goes wrong
    pub fn new() -> Result<Self, WorklogError> {
        let config = config::load()?;
        let client = Jira::from(&config.jira)?;

        let path = PathBuf::from(&config.application_data.local_worklog);

        if !path.exists() {
            println!("No support for the old journal. Use 'timesheet sync' to get your worklogs from Jira");
        }

        let worklog_service = WorklogStorage::new(&path)?;

        Ok(ApplicationRuntime {
            config,
            client,
            worklog_service,
        })
    }

    fn configuration(&self) -> &AppConfiguration {
        &self.config
    }

    pub fn jira_client(&self) -> &Jira {
        &self.client
    }

    pub fn worklog_service(&self) -> &WorklogStorage {
        &self.worklog_service
    }

    #[allow(clippy::missing_errors_doc)]
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
                Ok(OperationResult::Issues(issues))
            }
        }
    }

    #[allow(clippy::missing_errors_doc)]
    pub async fn sync_jira_issue_information(
        &self,
        issue_keys: &[JiraKey],
    ) -> Result<Vec<Issue>, WorklogError> {
        let jira_issues = self
            .jira_client()
            .get_issues_for_project(self.config.tracking_project.to_string())
            .await?;
        let result: Vec<Issue> = jira_issues
            .into_iter()
            .filter(|issue| issue_keys.contains(&issue.key))
            .collect();

        self.worklog_service().add_jira_issues(&result)?;
        Ok(result)
    }
}
