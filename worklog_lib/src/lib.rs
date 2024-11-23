use chrono::{DateTime, Local};
use common::config::AppConfiguration;
use common::journal::Journal;
use common::{config, WorklogError};
use jira_lib::{JiraClient, JiraKey};
use local_worklog::{LocalWorklog, LocalWorklogService};
use log::{debug, info, warn};
use std::fs;
use std::path::PathBuf;
use std::rc::Rc;

pub enum ApplicationRuntime {
    Production(RuntimeState),
    Test(RuntimeState)
}

pub struct RuntimeState {
    app_config: AppConfiguration,
    jira_client: JiraClient,
    local_worklog_service: local_worklog::LocalWorklogService,
}

impl ApplicationRuntime {
    ///
    /// # Errors
    /// Returns an error if the initialisation goes wrong
    pub fn new_production() -> Result<Self, WorklogError> {
        // Initialize the configuration using either the dbms path found in the
        // configuration file or the system default.
        let app_config = Self::init_config(None)?;
        let (jira_client, local_worklog_service) = Self::init_runtime(&app_config)?;

        Ok(ApplicationRuntime::Production(RuntimeState {
            app_config,
            jira_client,
            local_worklog_service
        }))
    }

    ///
    /// # Errors
    /// Returns an error if the initialisation of the runtime fails
    pub fn new_test() -> Result<Self, WorklogError> {
        let app_config = config::tmp_conf_load()?;
        debug!(
        "Creating runtime with this ApplicationConfig {:?}",
        &app_config
    );
        let (jira_client, local_worklog_service) = Self::init_runtime(&app_config)?;
        let test_runtime = ApplicationRuntime::Test(RuntimeState {
            app_config,
            jira_client,
            local_worklog_service
        });
        // Empty the database, if a previous run has left any data
        test_runtime
            .get_local_worklog_service()
            .purge_entire_local_worklog()?;

        Ok(test_runtime)
    }

    fn get_state(&self) -> &RuntimeState {
        match self {
            ApplicationRuntime::Production(state) |
            ApplicationRuntime::Test(state) => {
                state
            }
        }
    }
    pub fn get_application_configuration(&self) -> &AppConfiguration {
        &self.get_state().app_config
    }

    pub fn get_jira_client(&self) -> &JiraClient {
        &self.get_state().jira_client
    }

    pub fn get_journal(&self) -> Rc<dyn Journal> {
        // Just forward the invocation
        self.get_state().app_config.application_data.get_journal()
    }

    pub fn get_local_worklog_service(&self) -> &LocalWorklogService {
        &self.get_state().local_worklog_service
    }

    /// Load configuration from disk. If there is no local worklog dbms path in the configuration
    /// file, use the one provided or revert to the system default
    fn init_config(dbms_path: Option<&PathBuf>) -> Result<AppConfiguration, WorklogError> {
        let mut app_config = config::load()?;

        // If there is no path to the local_repo database in the configuration file,
        // use the default
        // TODO: Rewrite using match
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
                Some(dbms_path.unwrap().to_string_lossy().to_string());
        }

        Ok(app_config)
    }

    /// Initializes the runtime using the supplied application configuration
    fn init_runtime(
        app_config: &AppConfiguration,
    ) -> Result<(JiraClient, LocalWorklogService), WorklogError> {
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

        let local_worklog_service = LocalWorklogService::new(&path)?;
        Ok((jira_client, local_worklog_service))
    }
}


/// Migrates the data from the local journal into the local work log dbms by retrieving the
/// unique jira keys found in the journal and then downloading them from Jira.
/// # Errors
/// Returns a `WorklogError` if something goes wrong
///
#[allow(clippy::cast_possible_wrap)]
pub async fn migrate_csv_journal_to_local_worklog_dbms(
    start_after: Option<DateTime<Local>>,
) -> Result<i32, WorklogError> {

    let journal_file_name = config::journal_data_file_name();
    if !PathBuf::from(&journal_file_name).try_exists()? {
        eprintln!("Old journal not found so return");
        return Ok(0);
    }

    let runtime= ApplicationRuntime::new_production()?;
    // Find the unique keys in the local Journal
    let unique_keys = runtime.get_journal()
        .find_unique_keys()
        .map_err(|e| WorklogError::UniqueKeys(e.to_string()))?;

    eprintln!("Found these Jira keys in the old journal {unique_keys:?}");

    // For each Jira key
    for key in &unique_keys {
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

            let local_worklog = LocalWorklog::from_worklog(&wl, JiraKey::from(key.clone()));
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

    #[allow(clippy::cast_possible_truncation)]
    Ok(unique_keys.len() as i32)
}

/// Moves the local CSV journal file into a backup file
fn move_local_journal_to_backup_file(app_config: &AppConfiguration)
 -> Result<PathBuf, WorklogError> {

    let old_path = PathBuf::from(&app_config.application_data.journal_data_file_name);
    eprintln!("Checking to see if {} exists {:?}", &old_path.to_string_lossy(), &old_path.try_exists());

    match old_path.try_exists() {
        Ok(true) => {
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
        },
        Ok(false) => {        debug!("No CSV journal file found, continuing as normal. Everything OK");
            Err(WorklogError::FileNotFound(old_path.to_string_lossy().to_string())) // No migration required
        }
        Err(err) => {
            panic!("Unable to check if file {} exists: {}", old_path.to_string_lossy(), err);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_application_runtime() -> anyhow::Result<(), WorklogError> {
        let runtime = ApplicationRuntime::new_production()?;
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
        let test_runtime = ApplicationRuntime::new_test()?;
        let prod_runtime = ApplicationRuntime::new_production()?;

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
