use chrono::Local;
use common::config::ApplicationConfig;
use common::journal::Journal;
use common::{config, WorklogError};
use jira_lib::{JiraClient, JiraKey};
use local_worklog::{LocalWorklog, LocalWorklogService};
use log::debug;
use std::path::PathBuf;
use std::rc::Rc;

pub trait ApplicationRuntime {
    fn get_application_configuration(&self) -> &config::ApplicationConfig;
    fn get_jira_client(&self) -> &JiraClient;
    fn get_journal(&self) -> Rc<dyn Journal>;
    fn get_local_worklog_service(&self) -> &LocalWorklogService;
}
pub struct ApplicationProductionRuntime {
    app_config: ApplicationConfig,
    jira_client: JiraClient,
    local_worklog_service: local_worklog::LocalWorklogService,
}

impl ApplicationRuntime for ApplicationProductionRuntime {
    fn get_application_configuration(&self) -> &ApplicationConfig {
        &self.app_config
    }

    fn get_jira_client(&self) -> &JiraClient {
        &self.jira_client
    }

    fn get_journal(&self) -> Rc<dyn Journal> {
        self.app_config.application_data.get_journal()
    }

    fn get_local_worklog_service(&self) -> &LocalWorklogService {
        &self.local_worklog_service
    }
}

impl ApplicationProductionRuntime {
    pub fn new() -> Result<Box<dyn ApplicationRuntime>, WorklogError> {
        let app_config = Self::init_config(None)?;
        Self::init_runtime(app_config)
    }

    fn with_local_worklog_dbms_path(local_worklog_dbms_path: &PathBuf) -> Result<Box<dyn ApplicationRuntime>, WorklogError>{
        let mut app_config = Self::init_config(Some(local_worklog_dbms_path))?;
        Self::init_runtime(app_config)
    }

    /// Load configuration from disk. If there is no local worklog dbms path, use the one provided
    /// or revert to the system default
    fn init_config(dbms_path: Option<&PathBuf>) -> Result<ApplicationConfig, WorklogError> {
        let mut app_config = config::load()?;

        // If there is no path to the local_worklog database in the configuration file,
        // use the default
        if app_config.application_data.local_worklog.is_none() && dbms_path.is_none() {
            app_config.application_data.local_worklog = Some(
                config::local_worklog_dbms_file_name()
                    .to_string_lossy()
                    .to_string(),
            );
        } else {
            // Always override with a supplied database path
            app_config.application_data.local_worklog = Some(dbms_path.unwrap().to_string_lossy().to_string())
        }

        Ok(app_config)
    }

    /// Initializes the runtime using the supplied application configuration
    fn init_runtime(app_config: ApplicationConfig) -> Result<Box<dyn ApplicationRuntime>, WorklogError> {

        let jira_client = JiraClient::new(
            &app_config.jira.jira_url,
            &app_config.jira.user,
            &app_config.jira.token,
        )
            .map_err(|e| WorklogError::JiraClient { msg: e.to_string() })?;

        // Creates the Path to the local worklog DBMS
        let path = PathBuf::from(
            &app_config
                .application_data
                .local_worklog
                .as_ref()
                .unwrap()
                .clone(),
        );
        // Initialises our runtime
        let runtime = Box::new(ApplicationProductionRuntime {
            app_config,
            jira_client,
            local_worklog_service: LocalWorklogService::new(&path)?,
        });
        Ok(runtime)

    }
}

struct ApplicationTestRuntime {
}

impl ApplicationTestRuntime {
    pub fn new() -> Result<Box<dyn ApplicationRuntime>, WorklogError> {
        let path_buf = config::tmp_local_worklog_dbms_file_name()?;
        ApplicationProductionRuntime::with_local_worklog_dbms_path(&path_buf)
    }
}

async fn migrate_to_local_worklog_dbms(
    jira_client: &JiraClient,
    journal: &dyn Journal,
    local_worklog_service: &LocalWorklogService,
) -> Result<i32, WorklogError> {
    let unique_keys = journal
        .find_unique_keys()
        .map_err(|e| WorklogError::UniqueKeys(e.to_string()))?;

    for key in unique_keys.iter() {
        let work_logs = jira_client
            .get_worklogs_for_current_user(key, None)
            .await
            .map_err(|e| WorklogError::JiraResponse {
                msg: "get_worklogs_for_current_user() failed".into(),
                reason: e.to_string(),
            })?;

        for wl in work_logs {
            let wl = LocalWorklog {
                id: wl.id,
                issue_key: JiraKey(key.clone()),
                issueId: wl.issueId,
                author: wl.author.displayName,
                created: wl.created.with_timezone(&Local),
                updated: wl.updated.with_timezone(&Local),
                started: wl.started.with_timezone(&Local),
                timeSpent: wl.timeSpent,
                timeSpentSeconds: wl.timeSpentSeconds,
                comment: wl.comment,
            };
            debug!("Adding {:?} to local worklog DBMS", &wl);
            local_worklog_service.add_entry(wl)?;
        }
    }
    Ok(unique_keys.len() as i32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_application_runtime() -> anyhow::Result<(), WorklogError> {
        let runtime = ApplicationProductionRuntime::new()?;
        let application_config = runtime.get_application_configuration();
        assert!(
            application_config
                .application_data
                .journal_data_file_name
                .len()
                > 1,
            "Name of journal data is invalid {}",
            application_config.application_data.journal_data_file_name
        );
        let jira_client = runtime.get_jira_client();
        let user = jira_client.get_current_user().await;
        assert!(
            !user.display_name.is_empty(),
            "Seems like the get_current_user() call failed"
        );

        let _worklog = runtime.get_local_worklog_service();
        let _journal = runtime.get_journal();
        Ok(())
    }

    #[tokio::test]
    async fn test_migrate_to_local_worklog_dbms() -> anyhow::Result<(), WorklogError> {
        let test_runtime = ApplicationTestRuntime::new()?;
        let count = migrate_to_local_worklog_dbms(test_runtime.get_jira_client(), test_runtime.get_journal().as_ref(), &test_runtime.get_local_worklog_service()).await?;
        Ok(())
    }
}
