use crate::error::WorklogError;
use anyhow::Result;
use directories;
use directories::ProjectDirs;
use jira::config::JiraClientConfiguration;
use journal::csv::JournalCsv;
use journal::Journal;
use serde::{Deserialize, Serialize};
use std::error;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::rc::Rc;

#[cfg(target_os = "macos")]
use log::debug;
#[cfg(target_os = "macos")]
const KEYCHAIN_SERVICE: &str = "com.autostoresystem.jira_worklog";

/// Application configuration struct
/// Holds the data we need to connect to Jira, write to the local journal and so on
#[derive(Serialize, Deserialize, Debug, PartialEq, Default, Clone)]
pub struct AppConfiguration {
    /// Holds the URL to the Jira instance we are running again.
    pub jira: JiraClientConfiguration,

    /// Holds the project where issues to track time on are
    #[serde(default = "default_tracking_project")]
    pub tracking_project: String,

    /// This will ensure that the filename is created, even if the Toml file
    /// is an old version, which does not have an `application_data` section
    #[serde(default = "default_application_data")]
    pub application_data: ApplicationData,
}

/// Holds the configuration for the `application_data` section of the Toml file
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct ApplicationData {
    pub journal_data_file_name: String,
    /// The path to the local worklog data store
    pub local_worklog: Option<String>,
}

impl Default for ApplicationData {
    fn default() -> Self {
        ApplicationData {
            journal_data_file_name: journal_file().to_string_lossy().to_string(),
            local_worklog: Some(worklog_file().to_string_lossy().to_string()),
        }
    }
}

impl ApplicationData {
    #[must_use]
    pub fn get_journal(&self) -> Rc<dyn Journal> {
        // The trait object is created here and shoved onto the heap with a reference count, before
        // being returned
        Rc::new(JournalCsv::new(PathBuf::from(&self.journal_data_file_name)))
    }
}

/// Filename holding the application configuration parameters
#[must_use]
pub fn configuration_file() -> PathBuf {
    project_dirs().preference_dir().into()
}

/// Filename of the Sqlite DBMS holding the local repo of work logs
#[must_use]
pub fn worklog_file() -> PathBuf {
    project_dirs().data_dir().join("worklog.db")
}

#[allow(clippy::missing_errors_doc)]
#[allow(unused_mut)]
pub fn load() -> Result<AppConfiguration, WorklogError> {
    let config_path = configuration_file();

    let mut app_config = read(&config_path)?;

    #[cfg(target_os = "macos")]
    if cfg!(target_os = "macos") {
        // If the loaded configuration file holds a valid Jira token, migrate it to
        // the macOS Key Chain
        if app_config.jira.has_valid_jira_token()
            && secure_credentials::macos::get_secure_token(KEYCHAIN_SERVICE, &app_config.jira.user)
                .is_err()
        {
            create_configuration_file(&app_config, &config_path)
                .map_err(|_src_err| WorklogError::ConfigFileCreation { path: config_path })?;
        }

        // Merges the Jira token from the Keychain into the Application configuration
        merge_jira_token_from_keychain(&mut app_config);
    }
    Ok(app_config)
}

#[allow(clippy::missing_errors_doc)]
pub fn save(cfg: &AppConfiguration) -> Result<()> {
    create_configuration_file(cfg, &configuration_file())
}

#[allow(clippy::missing_errors_doc)]
pub fn remove() -> io::Result<()> {
    fs::remove_file(configuration_file().as_path())
}

/// Loads the current application configuration file or creates new one with default values
/// The location will be the system default as provided by `config_file_name()`
#[allow(clippy::missing_errors_doc)]
pub fn load_or_create() -> Result<AppConfiguration, Box<dyn error::Error>> {
    let p = configuration_file();
    if p.exists() && p.is_file() {
        Ok(load()?)
    } else {
        let cfg = AppConfiguration::default();
        create_configuration_file(&cfg, &configuration_file())?;
        Ok(cfg)
    }
}

#[allow(clippy::missing_errors_doc)]
pub fn application_config_to_string(cfg: &AppConfiguration) -> Result<String> {
    Ok(toml::to_string::<AppConfiguration>(cfg)?)
}

pub(crate) const JOURNAL_CSV_FILE_NAME: &str = "worklog_journal.csv";

/// Name of CSV file holding the local journal
#[must_use]
pub(crate) fn journal_file() -> PathBuf {
    project_dirs().data_dir().join(JOURNAL_CSV_FILE_NAME)
}

fn default_application_data() -> ApplicationData {
    ApplicationData::default()
}

fn default_tracking_project() -> String {
    "TIME".to_string()
}

fn project_dirs() -> ProjectDirs {
    ProjectDirs::from("com", "autostore", "jira_worklog")
        .expect("Unable to determine the name of the 'project_dirs' directory name")
}

/// Reads the `Application` configuration struct from the supplied TOML file
fn read(path: &Path) -> Result<AppConfiguration, WorklogError> {
    let mut file = File::open(path).map_err(|source| WorklogError::ApplicationConfig {
        path: path.into(),
        source,
    })?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .map_err(|source| WorklogError::ApplicationConfig {
            path: path.into(),
            source,
        })?;
    toml::from_str::<AppConfiguration>(&contents).map_err(|source| WorklogError::TomlParse {
        path: path.into(),
        source,
    })
}

#[allow(unused_mut)]
fn create_configuration_file(cfg: &AppConfiguration, path: &PathBuf) -> Result<()> {
    let directory = path.parent().unwrap();
    if !directory.try_exists()? {
        fs::create_dir_all(directory)?;
    }

    let mut cfg_updated = cfg.clone();

    #[cfg(target_os = "macos")]
    if cfg!(target_os = "macos") {
        debug!("MacOs: Moving security token into the keychain");
        migrate_jira_token_into_keychain(&mut cfg_updated);
    }

    let mut file = File::create(path)?;
    let toml = application_config_to_string(&cfg_updated)?;
    file.write_all(toml.as_bytes())?;

    Ok(())
}

/// Sets the Jira Access Security Token in the macOS Key Chain
/// See also the `security` command.
/// `
/// security add-generic-password -s com.autosstoresystem.jira_worklog \
///   -a steinar.cook@autostoresystem.com -w secure_token_goes_here
/// `
#[cfg(target_os = "macos")]
fn merge_jira_token_from_keychain(config: &mut AppConfiguration) {
    use log::warn;

    debug!("MacOS: retrieving the Jira access token from the keychain ...");
    match secure_credentials::macos::get_secure_token(KEYCHAIN_SERVICE, &config.jira.user) {
        Ok(token) => {
            debug!("Found Jira access token in keychain and injected it");
            config.jira.token = token;
        }
        Err(err) => {
            warn!(
                "No Jira Access Token in keychain for {} and {}",
                KEYCHAIN_SERVICE, &config.jira.user
            );
            warn!("ERROR: {err}");
            eprintln!(
                "No Jira Access Token in keychain for {} and {}",
                KEYCHAIN_SERVICE, &config.jira.user
            );
            eprintln!("If this is the first time your using the tool, this warning can be ignored");
        }
    }
}

const JIRA_TOKEN_STORED_IN_MACOS_KEYCHAIN: &str = "*** stored in macos keychain ***";

#[cfg(target_os = "macos")]
fn migrate_jira_token_into_keychain(app_config: &mut AppConfiguration) {
    match secure_credentials::macos::store_secure_token(
        KEYCHAIN_SERVICE,
        &app_config.jira.user,
        &app_config.jira.token,
    ) {
        Ok(()) => {
            debug!(
                "Jira access token stored into the Keychain under {} and {}",
                KEYCHAIN_SERVICE, app_config.jira.user
            );
            debug!("MacOs: Removing the security token from the config file");
        }
        #[allow(unused_variables)]
        Err(error) => {
            panic!("Unable to store the Jira access token into the MacOS keychain {error}");
        }
    }
    // a useless placeholder
    // This will ensure the jira security token in the config file on disk contains
    debug!("MacOs: Removing the security token from the config file");
    app_config.jira.token = JIRA_TOKEN_STORED_IN_MACOS_KEYCHAIN.to_string();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tom_parsing() {
        let toml_str = r#"
        [jira]
        jira_url = "http"
        user = "steinar"
        token = "rubbish"

        [application_data]
        journal_data_file_name = "journal"
        "#;

        let app_config: AppConfiguration = toml::from_str(toml_str).unwrap();
        assert_eq!(
            app_config.application_data.journal_data_file_name,
            "journal"
        );
    }

    /// Verifies that the `journal_data_file_name` is populated with a reasonable default even if it
    /// does not exist in the configuration file on disk
    #[test]
    fn test_toml_parsing_with_defaults_generated() {
        let toml_str = r#"
        [jira]
        jira_url = "http"
        user = "steinar"
        token = "rubbish"
        "#;

        let app_config: AppConfiguration = toml::from_str(toml_str).unwrap();
        assert_eq!(
            app_config.application_data.journal_data_file_name,
            journal_file().to_string_lossy()
        );
    }

    #[ignore]
    #[test]
    fn test_write_and_read_toml_file() -> Result<(), Box<dyn error::Error>> {
        let tmp_config_file = std::env::temp_dir().join("test-config.toml");

        let cfg = AppConfiguration::default();

        create_configuration_file(&cfg, &tmp_config_file)?;
        if let Ok(result) = read(&tmp_config_file) {
            // Don't compare the jira.token field as this may vary depending on operating system
            assert!(
                cfg.jira.jira_url == result.jira.jira_url
                    && cfg.jira.user == result.jira.user
                    && cfg.application_data == result.application_data
            );
        } else {
            panic!("Unable to read the TOML configuration back from disk");
        }

        Ok(())
    }

    #[test]
    fn test_jira_valid_token() {
        let mut app = AppConfiguration::default();
        assert!(!app.jira.has_valid_jira_token(), "{}", app.jira.token);

        app.jira.token = JIRA_TOKEN_STORED_IN_MACOS_KEYCHAIN.to_string();
        assert!(!app.jira.has_valid_jira_token());

        app.jira.token = "XXXXX3xFfGF07-XjakdCf_Y7_CNWuvhyHAhCr5sn4Q1kp35oUiN-zrZm9TeZUIllWqrMuPRc4Zcbo-GvCEgPZSjj1oUZkUZBc7vEOJmSxcdq-lEWHkECvyAee64iBboDeYDJZIaiAidS57YJQnWCEAADmGnE5TyDeZqRkdMgvbMvU9Wyd6T05wI=3FF0BE2A".to_string();
        assert!(app.jira.has_valid_jira_token());
    }
}
