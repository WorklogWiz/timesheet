use chrono::{DateTime, Days, Local};
use common::config::ApplicationConfig;
use common::journal::Journal;
use common::{config, WorklogError};
use jira_lib::{JiraClient, JiraKey, Worklog};
use local_worklog::{LocalWorklog, LocalWorklogService};
use log::{debug, info, warn};
use std::fs;
use std::path::PathBuf;
use std::process::exit;
use std::ptr::replace;
use std::rc::Rc;
use std::sync::Arc;
use tokio::runtime::{Handle, Runtime};

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
        // Just forward the invocation
        self.app_config.application_data.get_journal()
    }

    fn get_local_worklog_service(&self) -> &LocalWorklogService {
        &self.local_worklog_service
    }
}

impl ApplicationProductionRuntime {
    pub fn new() -> Result<Box<dyn ApplicationRuntime>, WorklogError> {
        // Initialize the configuration using either the dbms path found in the
        // configuration file or the system default.
        let app_config = Self::init_config(None)?;
        Self::init_runtime(app_config)
    }

    /// Only available to unit tests, which requires access to a local test database
    fn with_local_worklog_dbms_path(
        local_worklog_dbms_path: &PathBuf,
    ) -> Result<Box<dyn ApplicationRuntime>, WorklogError> {
        let mut app_config = Self::init_config(Some(local_worklog_dbms_path))?;
        Self::init_runtime(app_config)
    }

    pub fn with_app_config(
        app_config: ApplicationConfig,
    ) -> Result<Box<dyn ApplicationRuntime>, WorklogError> {
        Self::init_runtime(app_config)
    }

    /// Load configuration from disk. If there is no local worklog dbms path in the configuration
    /// file, use the one provided or revert to the system default
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
        } else if dbms_path.is_some() {
            debug!(
                "Using {} as the local worklog data store",
                dbms_path.as_ref().unwrap().to_string_lossy()
            );
            // Always override when there is a supplied database path
            app_config.application_data.local_worklog =
                Some(dbms_path.unwrap().to_string_lossy().to_string())
        }

        Ok(app_config)
    }

    /// Initializes the runtime using the supplied application configuration
    fn init_runtime(
        app_config: ApplicationConfig,
    ) -> Result<Box<dyn ApplicationRuntime>, WorklogError> {
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

/// Migrates the data from the local journal into the local work log dbms by retrieving the
/// unique jira keys found in the journal and then downloading them from Jira.
pub async fn migrate_csv_journal_to_local_worklog_dbms(
    runtime: &dyn ApplicationRuntime,
    start_after: Option<DateTime<Local>>,
) -> Result<i32, WorklogError> {
    eprintln!("migrate_csv_journal_to_local_worklog_dbms() :- entering..");
    debug!("Debug is working!!");

    let journal_file_name = config::journal_data_file_name();
    if !PathBuf::from(&journal_file_name).try_exists()? {
        eprintln!("Old journal not found so return");
        return Ok(0);
    }

    // Find the unique keys in the local Journal
    let unique_keys = runtime.get_journal()
        .find_unique_keys()
        .map_err(|e| WorklogError::UniqueKeys(e.to_string()))?;

    eprintln!("Found these Jira keys in the old journal {:?}", unique_keys);

    // For each Jira key
    for key in unique_keys.iter() {
        // Get the work log entries
        let key_copy = key.clone();
        debug!("Retrieving worklogs for current user for key {}", &key);

        let work_logs = runtime.get_jira_client()
            .get_worklogs_for_current_user(&key_copy, start_after)
            .await
            .map_err(|e| WorklogError::JiraResponse {
                msg: "get_worklogs_for_current_user() failed".into(),
                reason: e.to_string(),
            })?;

        //let work_logs = handle.block_on(async move { work_logs_join_handle.await.unwrap()})?;
        // ... and stuff them into our local dbms
        debug!(
            "Inserting {} entries into the local worklog database {}",
            work_logs.len(),
            runtime.get_local_worklog_service().get_dbms_path().to_string_lossy()
        );

        for wl in work_logs {

            let local_worklog = LocalWorklog::from_worklog(&wl, JiraKey(key.clone()));
            debug!("Adding {:?} to local worklog DBMS", &local_worklog);
            if let Err(error) = runtime.get_local_worklog_service().add_entry(&local_worklog){
                warn!("Failed to insert {:?} : {}", local_worklog, error);
                info!("Continuing with next entry");
            }
        }
    }
    debug!("Going to move the local journal to a backup file");
    // Moves the local journal file into a safe spot
    move_local_journal_to_backup_file(runtime.get_application_configuration())?;

    Ok(unique_keys.len() as i32)
}

/// Moves the local CSV journal file into a backup file
fn move_local_journal_to_backup_file(app_config: &ApplicationConfig)
 -> Result<PathBuf, WorklogError> {

    let old_path = PathBuf::from(&app_config.application_data.journal_data_file_name);
    eprintln!("Checking to see if {} exists {:?}", &old_path.to_string_lossy(), &old_path.try_exists());

    if old_path.try_exists()? {
        debug!(
                "An existing journal found at {}, migrating to local DBMS in {}",
                &old_path.to_string_lossy(),
                &app_config
                    .application_data
                    .local_worklog
                    .as_ref()
                    .unwrap()
            );

        let mut new_path = old_path.clone();
        new_path.set_file_name(config::JOURNAL_CSV_FILE_NAME.to_string().replace(".csv", "-backup.csv"));
        fs::rename(&old_path, &new_path)?;
        eprintln!("Renamed to {}", &new_path.clone().to_string_lossy());

        Ok(new_path) // Migration performed
    } else {
        debug!("No CSV journal file found, continuing as normal. Everything OK");
        Err(WorklogError::FileNotFound(old_path.to_string_lossy().to_string())) // No migration required
    }
}


pub struct ApplicationTestRuntime {}

impl ApplicationTestRuntime {
    pub fn new() -> Result<Box<dyn ApplicationRuntime>, WorklogError> {
        let app_config = config::tmp_conf_load()?;
        debug!(
            "Creating runtime with this ApplicationConfig {:?}",
            &app_config
        );
        let test_runtime = ApplicationProductionRuntime::with_app_config(app_config.clone())?;

        // Empty the database, if a previous run has left any data
        test_runtime
            .get_local_worklog_service()
            .purge_entire_local_worklog()?;

        Ok(test_runtime)
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;
    use common::journal::Entry;

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


    #[test]
    fn test_runtimes() -> anyhow::Result<(), WorklogError> {
        let test_runtime = ApplicationTestRuntime::new()?;
        let prod_runtime = ApplicationProductionRuntime::new()?;

        assert_ne!(
            test_runtime.get_application_configuration(),
            prod_runtime.get_application_configuration()
        );
        assert_eq!(
            test_runtime
                .get_application_configuration()
                .application_data
                .journal_data_file_name,
            config::tmp_journal_data_file_name()
                .to_string_lossy()
                .to_string()
        );
        assert_ne!(
            test_runtime.get_local_worklog_service().get_dbms_path(),
            prod_runtime.get_local_worklog_service().get_dbms_path()
        );
        Ok(())
    }
}
