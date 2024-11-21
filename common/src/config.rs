use crate::journal::journal_csv::JournalCsv;
use crate::journal::Journal;
use crate::WorklogError;
use anyhow::Context;
use anyhow::Result;
use directories;
use directories::ProjectDirs;
use log::{debug};
use serde::{Deserialize, Serialize};
use std::error;
use std::fs::{self, remove_file, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::rc::Rc;

const KEYCHAIN_SERVICE: &str = "com.autostoresystem.jira_worklog";
/// Application configuration struct
/// Holds the data we need to connect to Jira, write to the local journal and so on
#[derive(Serialize, Deserialize, Debug, PartialEq, Default, Clone)]
pub struct ApplicationConfig {
    /// Holds the URL to the Jira instance we are running again.
    pub jira: Jira,
    /// This will ensure that the filename is created, even if the Toml file
    /// is an old version, which does not have an `application_data` section
    #[serde(default = "default_application_data_section")]
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
            journal_data_file_name: journal_data_file_name().to_string_lossy().to_string(),
            local_worklog: Some(local_worklog_dbms_file_name().to_string_lossy().to_string()),
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

fn default_application_data_section() -> ApplicationData {
    ApplicationData::default()
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Jira {
    pub jira_url: String,
    pub user: String,
    pub token: String,
}

impl Default for Jira {
    fn default() -> Self {
        Jira {
            jira_url: "https://autostore.atlassian.net/rest/api/latest".into(),
            user: "user.name@autostoresystem.com".into(),
            token: "<your secret jira token goes here>".into(),
        }
    }
}

impl Jira {
    /// Does the token look like a valid Jira Security token?
    #[must_use]
    pub fn has_valid_jira_token(&self) -> bool {
        !(self.token == Jira::default().token
            || self.token.contains("secret")
            || self.token == JIRA_TOKEN_STORED_IN_MACOS_KEYCHAIN)
    }
}
/// Filename holding the application configuration parameters
#[must_use]
pub fn config_file_name() -> PathBuf {
    project_dirs().preference_dir().into()
}

/// Creates a temporary configuration file name, which is useful for integration tests
pub fn tmp_config_file_name() -> PathBuf {
    tmp_dir().join("worklog_conf.toml").into()
}

pub const JOURNAL_CSV_FILE_NAME: &'static str = "worklog_journal.csv";

/// Name of CSV file holding the local journal
#[must_use]
pub fn journal_data_file_name() -> PathBuf {
    project_dirs().data_dir().join(JOURNAL_CSV_FILE_NAME)
}

pub fn tmp_journal_data_file_name() -> PathBuf {
    tmp_dir().join(JOURNAL_CSV_FILE_NAME)
}

/// Filename of the Sqlite DBMS holding the local repo of work logs
#[must_use]
pub fn local_worklog_dbms_file_name() -> PathBuf {
    project_dirs().data_dir().join("worklog.db")
}

/// Creates a temporary local Sqlite DBMS file name, which is quite useful for integration tests
pub fn tmp_local_worklog_dbms_file_name() -> anyhow::Result<PathBuf,WorklogError> {
    // Create a temporary file with a custom prefix
    let temp_file = tempfile::Builder::new()
        .prefix("worklog")
        .suffix(".db")
        .tempfile()
        .expect("Failed to create temporary file");

    let tmp_db = PathBuf::from(temp_file.path());
    if tmp_db.try_exists()? {
        let _result = remove_file(&tmp_db).with_context(|| {
            format!(
                "Unable to remove database file {}",
                &tmp_db.to_string_lossy()
            )
        });
        if let Ok(true) = tmp_db.try_exists() {
            return Err(WorklogError::FileNotDeleted(tmp_db.to_string_lossy().to_string()));
        }
    } else {
        // Create the directory if it doesn't exist
        fs::create_dir_all(&tmp_db.parent().unwrap()).map_err(|e| WorklogError::CreateDir(e))?;
    }
    Ok(tmp_db)
}

pub fn tmp_dir() -> PathBuf {
    project_dirs().cache_dir().into()
}

fn project_dirs() -> ProjectDirs {
    ProjectDirs::from("com", "autostore", "jira_worklog")
        .expect("Unable to determine the name of the 'project_dirs' directory name")
}

/// Reads the `Application` configuration struct from the supplied TOML file
fn read(path: &Path) -> Result<ApplicationConfig, WorklogError> {
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
    Ok(
        toml::from_str::<ApplicationConfig>(&contents).map_err(|source| {
            WorklogError::TomlParse {
                path: path.into(),
                source: source.into(),
            }
        })?,
    )
}

fn create_configuration_file(cfg: &ApplicationConfig, path: &PathBuf) -> Result<()> {
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

#[allow(clippy::missing_errors_doc)]
pub fn load() -> Result<ApplicationConfig, WorklogError> {
    let config_path = config_file_name();

    let mut app_config = read(&config_path)?;

    #[cfg(target_os = "macos")]
    if cfg!(target_os = "macos") {
        // If the loaded configuration file holds a valid Jira token, migrate it to
        // the macOS Key Chain
        if app_config.jira.has_valid_jira_token()
            && secure_credentials::get_secure_token(KEYCHAIN_SERVICE, &app_config.jira.user)
                .is_err()
        {
            create_configuration_file(&app_config, &config_path).map_err(|_src_err| {
                WorklogError::ConfigFileCreation {
                    path: config_path.into(),
                }
            })?;
        }

        // Merges the Jira token from the Keychain into the Application configuration
        merge_jira_token_from_keychain(&mut app_config);
    }
    Ok(app_config)
}

pub fn tmp_conf_load() -> Result<ApplicationConfig, WorklogError> {
    let mut application_config = load()?;

    if application_config.jira.has_valid_jira_token() {
        let config_file_name = tmp_config_file_name();

        application_config.application_data.journal_data_file_name = tmp_journal_data_file_name().to_string_lossy().to_string();
        eprintln!("{}", application_config.application_data.journal_data_file_name);
        application_config.application_data.local_worklog = Some(tmp_local_worklog_dbms_file_name().unwrap().to_string_lossy().to_string());
        create_configuration_file(&application_config, &config_file_name).expect(format!("Unable to create configuration file {}, with this content: {:?}", config_file_name.to_string_lossy(), application_config).as_str());
        Ok(application_config)
    } else {
        panic!("The Jira token in the application configuration is invalid. You need to create a configuration file with a valid Jira token");
    }
}

#[allow(clippy::missing_errors_doc)]
pub fn save(cfg: &ApplicationConfig) -> Result<()> {
    create_configuration_file(cfg, &config_file_name())
}

#[allow(clippy::missing_errors_doc)]
pub fn remove() -> io::Result<()> {
    fs::remove_file(config_file_name().as_path())
}

/// Loads the current application configuration file or creates new one with default values
/// The location will be the system default as provided by `config_file_name()`
#[allow(clippy::missing_errors_doc)]
pub fn load_or_create() -> Result<ApplicationConfig, Box<dyn error::Error>> {
    let p = config_file_name();
    if p.exists() && p.is_file() {
        Ok(load()?)
    } else {
        let cfg = ApplicationConfig::default();
        create_configuration_file(&cfg, &config_file_name())?;
        Ok(cfg)
    }
}

/// Sets the Jira Access Security Token in the macOS Key Chain
/// See also the `security` command.
/// `
/// security add-generic-password -s com.autosstoresystem.jira_worklog \
///   -a steinar.cook@autostoresystem.com -w secure_token_goes_here
/// `
#[cfg(target_os = "macos")]
fn merge_jira_token_from_keychain(config: &mut ApplicationConfig) {
    use log::warn;

    debug!("MacOS: retrieving the Jira access token from the keychain ...");
    match secure_credentials::get_secure_token(KEYCHAIN_SERVICE, &config.jira.user) {
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
fn migrate_jira_token_into_keychain(app_config: &mut ApplicationConfig) {
    match secure_credentials::store_secure_token(
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

#[allow(clippy::missing_errors_doc)]
pub fn application_config_to_string(cfg: &ApplicationConfig) -> Result<String> {
    Ok(toml::to_string::<ApplicationConfig>(cfg)?)
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

        let app_config: ApplicationConfig = toml::from_str(toml_str).unwrap();
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

        let app_config: ApplicationConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(
            app_config.application_data.journal_data_file_name,
            journal_data_file_name().to_string_lossy()
        );
    }

    #[test]
    fn test_write_and_read_toml_file() -> Result<(), Box<dyn error::Error>> {
        let config_file_path = config_file_name().clone();
        let file_name = config_file_path.file_name().unwrap();

        let tmp_config_file = std::env::temp_dir().join(file_name);

        let cfg = ApplicationConfig::default();
        if cfg!(target_os = "macos") {
            assert!(&cfg
                .application_data
                .journal_data_file_name
                .contains("Application Support"));
        }

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
    fn test_data_dir_on_mac() {
        let p = journal_data_file_name();

        if cfg!(target_os = "macos") {
            assert!(p
                .to_string_lossy()
                .to_string()
                .contains("Application Support"));
        }

        eprintln!("{:?}", p.to_string_lossy());
    }

    #[test]
    fn test_jira_valid_token() {
        let mut app = ApplicationConfig::default();
        assert!(!app.jira.has_valid_jira_token(), "{}", app.jira.token);

        app.jira.token = JIRA_TOKEN_STORED_IN_MACOS_KEYCHAIN.to_string();
        assert!(!app.jira.has_valid_jira_token());

        app.jira.token = "XXXXX3xFfGF07-XjakdCf_Y7_CNWuvhyHAhCr5sn4Q1kp35oUiN-zrZm9TeZUIllWqrMuPRc4Zcbo-GvCEgPZSjj1oUZkUZBc7vEOJmSxcdq-lEWHkECvyAee64iBboDeYDJZIaiAidS57YJQnWCEAADmGnE5TyDeZqRkdMgvbMvU9Wyd6T05wI=3FF0BE2A".to_string();
        assert!(app.jira.has_valid_jira_token());
    }

    #[test]
    fn test_tmp_conf_load() {
        let config = tmp_conf_load().expect("Unable to create a temporary configuration");
    }
}
