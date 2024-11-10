use std::path::PathBuf;
use common::config::ApplicationConfig;
use common::journal::Journal;
use common::{config, WorklogError};
use jira_lib::JiraClient;
use local_worklog::LocalWorklogService;

pub trait ApplicationRuntime {
    fn get_application_configuration(&self) -> &config::ApplicationConfig;
    fn get_jira_client(&self) -> &JiraClient;
    fn get_journal(&self) -> Box<dyn Journal>;
    fn get_local_worklog(&self) -> &LocalWorklogService;
}
pub struct ApplicationProductionRuntime {
    app_config: ApplicationConfig,
    jira_client: JiraClient,
    local_worklog: local_worklog::LocalWorklogService,
}

impl ApplicationRuntime for ApplicationProductionRuntime {
    fn get_application_configuration(&self) -> &ApplicationConfig {
        &self.app_config
    }

    fn get_jira_client(&self) -> &JiraClient {
        &self.jira_client
    }

    fn get_journal(&self) -> Box<dyn Journal> {
        self.app_config.application_data.get_journal()
    }

    fn get_local_worklog(&self) -> &LocalWorklogService {
        &self.local_worklog
    }
}

impl ApplicationProductionRuntime {
    pub fn new() -> Result<Box<dyn ApplicationRuntime>, WorklogError> {
        let app_config = config::load()?;
        let jira_client = JiraClient::new(
            &app_config.jira.jira_url,
            &app_config.jira.user,
            &app_config.jira.token,
        )
        .map_err(|e| WorklogError::JiraClient { msg: e.to_string() })?;

        let string = &app_config.application_data.local_worklog.as_ref().unwrap().clone();
        let path = PathBuf::from(string);
        let runtime = Box::new(ApplicationProductionRuntime {
            app_config,
            jira_client,
            local_worklog: LocalWorklogService::new(&path)?
        });
        Ok(runtime)
    }
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
                > 1
        );
        let jira_client = runtime.get_jira_client();
        let user = jira_client.get_current_user().await;
        assert!(
            !user.display_name.is_empty(),
            "Seems like the get_current_user() call failed"
        );

        let _worklog = runtime.get_local_worklog();
        let _journal = runtime.get_journal();
        Ok(())
    }
}
