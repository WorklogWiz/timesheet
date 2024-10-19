use std::fs::{create_dir_all, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::exit;
use std::{fs, io};

use directories;
use directories::ProjectDirs;
use log::{debug,  warn};
use serde::{Deserialize, Serialize};

const KEYCHAIN_SERVICE: &str = "com.autostoresystem.jira_worklog";
/// Application configuration struct
/// Holds the data we need to connect to Jira, write to the local journal and so on
#[derive(Serialize, Deserialize, Debug, PartialEq, Default, Clone)]
pub struct Application {
    /// Holds the URL to the Jira instance we are running agains.
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
}

impl Default for ApplicationData {
    fn default() -> Self {
        ApplicationData {
            journal_data_file_name: journal_data_file_name().to_string_lossy().to_string(),
        }
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
            jira_url: "https://autostore.atlassian.net/rest/api/latest".to_string(),
            user: "user.name@autostoresystem.com".to_string(),
            token: "<your secret Jira token goes here>".to_string(),
        }
    }
}

impl Jira {
    /// Does the token look like a valid Jira Security token
    #[must_use]
    pub fn has_valid_jira_token(&self) -> bool {
        !(self.token == Jira::default().token
            || self.token.contains("secret")
            || self.token == JIRA_TOKEN_STORED_IN_MACOS_KEYCHAIN)
    }
}

#[must_use]
pub fn file_name() -> PathBuf {
    let project_dirs = project_dirs();
    let p = project_dirs.preference_dir();
    PathBuf::from(p)
}

#[must_use]
pub fn journal_data_file_name() -> PathBuf {
    let p = project_dirs();
    let data_dir = p.data_dir();
    data_dir.join("worklog_journal.csv")
}

fn project_dirs() -> ProjectDirs {
    ProjectDirs::from("com", "autostore", "jira_worklog")
        .expect("Unable to determine the name of the 'project_dirs' directory name")
}

/// Assumes there is a configuration file and loads it
///
/// # Panics
///
/// Will panic if the configuration file could not be written after migrating the
/// secure Jira Token from the configuration file into the macos keychain
#[allow(clippy::missing_errors_doc)]
pub fn load_configuration() -> Result<Application, io::Error> {
    let config_path = file_name();

    let mut app_config = match read_configuration(&config_path) {
        Ok(config) => config,
        Err(err) => {
            eprintln!("ERROR: Unable to parse {}", config_path.to_string_lossy());
            eprintln!("Cause: {err:?}");
            exit(4);
        }
    };

    #[cfg(target_os = "macos")]
    if cfg!(target_os = "macos") {
        // If the loaded configuration file holds a valid Jira token, migrate it to
        // the macos Keychain
        if app_config.jira.has_valid_jira_token()
            && secure_credentials::get_secure_token(KEYCHAIN_SERVICE, &app_config.jira.user).is_err()
        {
            #[allow(clippy::manual_assert)]
            if create_configuration_file(&app_config, &config_path).is_err() {
                panic!("Unable to migrate the Jira token from the config file to keychain");
            }
        }
        // Merges the Jira token from the Keychain into the Application configuration
        merge_jira_token_from_keychain(&mut app_config);
    }
    Ok(app_config)
}

#[allow(clippy::missing_errors_doc)]
pub fn save_configuration(application_config: &Application) -> io::Result<()> {
    create_configuration_file(application_config, &file_name())
}

#[allow(clippy::missing_errors_doc)]
pub fn remove_configuration() -> io::Result<()> {
    fs::remove_file(file_name().as_path())
}

#[allow(clippy::missing_errors_doc)]
fn create_and_save_sample_configuration() -> Result<Application, io::Error> {
    debug!("create_and_save_sample_configuration() :- entering ...");
    let application_config = Application::default();
    create_configuration_file(&application_config, &file_name())?;
    Ok(application_config)
}

#[allow(clippy::missing_errors_doc)]
pub fn load_or_create_configuration() -> Result<Application, io::Error> {
    debug!("Loading or creating the config file");

    match is_configuration_file_available() {
        None => create_and_save_sample_configuration(),
        Some(app_config) => Ok(app_config),
    }
}

#[allow(clippy::missing_panics_doc)]
#[must_use]
fn is_configuration_file_available() -> Option<Application> {
    let p = file_name();
    if p.exists() && p.is_file() {
        match load_configuration() {
            Ok(app_config) => Some(app_config),
            Err(e) => {
                panic!(
                    "Unable to load the configuration file from {}, reason: {}",
                    file_name().to_string_lossy(),
                    e
                )
            }
        }
    } else {
        None
    }
}

fn read_configuration(path: &Path) -> Result<Application, io::Error> {
    let mut file = match File::open(path) {
        Ok(f) => f,
        Err(e) => {
            debug!("Unable to open {}, cause: {}", path.to_string_lossy(), e);
            return Err(e);
        }
    };

    let mut contents = String::new();
    file.read_to_string(&mut contents)?;

    match toml::from_str::<Application>(&contents) {
        Ok(config) => Ok(config),
        Err(err) => {
            eprintln!("ERROR: Unable to parse {}", path.to_string_lossy());
            eprintln!("Cause: {err:?}");

            exit(4);
        }
    }
}

/// Sets the Jira Access Security Token in the macos Keychain
/// See also the `security` command.
/// `
/// security add-generic-password -s com.autosstoresystem.jira_worklog \
///   -a steinar.cook@autostoresystem.com -w secure_token_goes_here
/// `
#[cfg(target_os = "macos")]
fn merge_jira_token_from_keychain(config: &mut Application) {
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

#[allow(clippy::manual_assert)]
fn create_configuration_file(
    application_config: &Application,
    path: &PathBuf,
) -> Result<(), io::Error> {
    debug!(
        "create_configuration_file({}) :- entering ..",
        path.to_string_lossy()
    );

    let directory = path.parent().unwrap();
    if directory.try_exists()? {
        debug!("Path {} exists", directory.to_string_lossy());
    } else {
        create_dir_all(directory)?;
    }

    match path.parent() {
        None => {} // Root directory ??
        Some(parent) => match create_dir_all(parent) {
            Err(e) => {
                panic!(
                    "Unable to recursively create directory {}, cause: {}",
                    parent.to_string_lossy(),
                    e
                )
            }
            Ok(()) => {
                if !parent.is_dir() {
                    panic!(
                        "Interesting, directory {} created, but it does not exist!",
                        parent.to_string_lossy()
                    );
                }
            }
        },
    }
    let mut config_updated = application_config.clone();

    #[cfg(target_os = "macos")]
    if cfg!(target_os = "macos") {
        debug!("MacOs: Moving security token into the keychain");
        migrate_jira_token_into_keychain(&mut config_updated);
    }

    let mut file = File::create(path)?;
    let toml = application_config_to_string(&config_updated);
    file.write_all(toml.as_bytes())?;

    Ok(())
}

const JIRA_TOKEN_STORED_IN_MACOS_KEYCHAIN: &str = "*** stored in macos keychain ***";

#[cfg(target_os = "macos")]
fn migrate_jira_token_into_keychain(app_config: &mut Application) {
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
            panic!(
                "Unable to store the Jira access token into the MacOS keychain {error}"
            );
        }
    }
    // a useless placeholder
    // This will ensure the jira security token in the config file on disk contains
    debug!("MacOs: Removing the security token from the config file");
    app_config.jira.token = JIRA_TOKEN_STORED_IN_MACOS_KEYCHAIN.to_string();
}

#[allow(clippy::missing_panics_doc)]
#[must_use]
pub fn application_config_to_string(application_config: &Application) -> String {
    match toml::to_string::<Application>(application_config) {
        Ok(s) => s,
        #[allow(unused_variables)]
        Err(e) => panic!(
            "Unable to transform application config {application_config:?} structure into Toml: {e}"
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(target_os = "macos")]
    const MAC_OS_APP_DATA_DIR: &str = "Application Support";

    #[test]
    pub fn test_load_configuration() {
        let config_result = load_configuration();
        assert!(
            config_result.is_ok(),
            "Unable to load {}",
            file_name().to_string_lossy()
        );

        let config = config_result.unwrap();
        println!("Config: {config:?}");
    }

    #[test]
    fn test_tom_parsing() {
        let toml_str = r#"
        [jira]
        jira_url = "http"
        user = "steinar"
        token = "rubbish"

        [dbms]
        connect = "some postgres gibberish"

        [application_data]
        journal_data_file_name = "journal"
        "#;

        let app_config: Application = toml::from_str(toml_str).unwrap();
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

        [dbms]
        connect = "some postgres gibberish"
        "#;

        let app_config: Application = toml::from_str(toml_str).unwrap();
        assert_eq!(
            app_config.application_data.journal_data_file_name,
            journal_data_file_name().to_string_lossy()
        );
    }

    #[test]
    fn test_write_and_read_toml_file() -> Result<(), io::Error> {
        let config_file_path = file_name().clone();
        let file_name = config_file_path.file_name().unwrap();

        let tmp_config_file = std::env::temp_dir().join(file_name);

        let application_config = Application::default();
        if cfg!(target_os = "macos") {
            assert!(&application_config
                .application_data
                .journal_data_file_name
                .contains(MAC_OS_APP_DATA_DIR));
        }

        create_configuration_file(&application_config, &tmp_config_file)?;
        if let Ok(result) = read_configuration(&tmp_config_file) {
            assert_eq!(&application_config, &result);
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
        let mut app = Application::default();
        assert_eq!(app.jira.has_valid_jira_token(), false, "{}", app.jira.token);

        app.jira.token = JIRA_TOKEN_STORED_IN_MACOS_KEYCHAIN.to_string();
        assert_eq!(app.jira.has_valid_jira_token(), false);

        app.jira.token = "XXXXX3xFfGF07-XjakdCf_Y7_CNWuvhyHAhCr5sn4Q1kp35oUiN-zrZm9TeZUIllWqrMuPRc4Zcbo-GvCEgPZSjj1oUZkUZBc7vEOJmSxcdq-lEWHkECvyAee64iBboDeYDJZIaiAidS57YJQnWCEAADmGnE5TyDeZqRkdMgvbMvU9Wyd6T05wI=3FF0BE2A".to_string();
        assert!(app.jira.has_valid_jira_token());
    }
}
