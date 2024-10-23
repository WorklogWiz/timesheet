use directories;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::error;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use log::debug;
use crate::journal::{Journal, JournalCsv};

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

impl ApplicationData {
    pub fn get_journal(&self) -> Box<dyn Journal> {
        Box::new(JournalCsv::new(PathBuf::from(&self.journal_data_file_name)))
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

#[must_use]
pub fn file_name() -> PathBuf {
    project_dirs()
        .preference_dir()
        .into()
}

#[must_use]
pub fn journal_data_file_name() -> PathBuf {
    project_dirs()
        .data_dir()
        .join("worklog_journal.csv")
}

fn project_dirs() -> ProjectDirs {
    ProjectDirs::from("com", "autostore", "jira_worklog")
        .expect("Unable to determine the name of the 'project_dirs' directory name")
}

fn read(path: &Path) -> Result<Application, Box<dyn error::Error>> {
    let mut file = File::open(path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    Ok(toml::from_str::<Application>(&contents)?)
}

fn create_configuration_file(
    cfg: &Application,
    path: &PathBuf
) -> Result<(), Box<dyn error::Error>> {
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
pub fn load() -> Result<Application, Box<dyn error::Error>> {
    let config_path = file_name();

    let mut app_config = read(&config_path)?;

    #[cfg(target_os = "macos")]
    if cfg!(target_os = "macos") {
        // If the loaded configuration file holds a valid Jira token, migrate it to
        // the macos Key Chain
        if app_config.jira.has_valid_jira_token()
            && secure_credentials::get_secure_token(KEYCHAIN_SERVICE, &app_config.jira.user).is_err()
        {
            create_configuration_file(&app_config, &config_path)?;
        }

        // Merges the Jira token from the Keychain into the Application configuration
        merge_jira_token_from_keychain(&mut app_config);
    }
    Ok(app_config)
}

#[allow(clippy::missing_errors_doc)]
pub fn save(cfg: &Application) -> Result<(), Box<dyn error::Error>> {
    create_configuration_file(cfg, &file_name())
}

#[allow(clippy::missing_errors_doc)]
pub fn remove() -> io::Result<()> {
    fs::remove_file(file_name().as_path())
}

#[allow(clippy::missing_errors_doc)]
pub fn load_or_create() -> Result<Application, Box<dyn error::Error>> {
    let p = file_name();
    if p.exists() && p.is_file() {
        Ok(load()?)
    } else {
        let cfg = Application::default();
        create_configuration_file(&cfg, &file_name())?;
        Ok(cfg)
    }
}

/// Sets the Jira Access Security Token in the macos Key Chain
/// See also the `security` command.
/// `
/// security add-generic-password -s com.autosstoresystem.jira_worklog \
///   -a steinar.cook@autostoresystem.com -w secure_token_goes_here
/// `
#[cfg(target_os = "macos")]
fn merge_jira_token_from_keychain(config: &mut Application) {
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

#[allow(clippy::missing_errors_doc)]
pub fn application_config_to_string(cfg: &Application) -> Result<String, Box<dyn error::Error>> {
    Ok(toml::to_string::<Application>(cfg)?)
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
        "#;

        let app_config: Application = toml::from_str(toml_str).unwrap();
        assert_eq!(
            app_config.application_data.journal_data_file_name,
            journal_data_file_name().to_string_lossy()
        );
    }

    #[test]
    fn test_write_and_read_toml_file() -> Result<(), Box<dyn error::Error>> {
        let config_file_path = file_name().clone();
        let file_name = config_file_path.file_name().unwrap();

        let tmp_config_file = std::env::temp_dir().join(file_name);

        let cfg = Application::default();
        if cfg!(target_os = "macos") {
            assert!(&cfg
                .application_data
                .journal_data_file_name
                .contains("Application Support"));
        }

        create_configuration_file(&cfg, &tmp_config_file)?;
        if let Ok(result) = read(&tmp_config_file) {
            // Don't compare the jira.token field as this may vary depending on operating system
            assert!(cfg.jira.jira_url == result.jira.jira_url
                && cfg.jira.user == result.jira.user
                && cfg.application_data == result.application_data);
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
        assert!(!app.jira.has_valid_jira_token(), "{}", app.jira.token);

        app.jira.token = JIRA_TOKEN_STORED_IN_MACOS_KEYCHAIN.to_string();
        assert!(!app.jira.has_valid_jira_token());

        app.jira.token = "XXXXX3xFfGF07-XjakdCf_Y7_CNWuvhyHAhCr5sn4Q1kp35oUiN-zrZm9TeZUIllWqrMuPRc4Zcbo-GvCEgPZSjj1oUZkUZBc7vEOJmSxcdq-lEWHkECvyAee64iBboDeYDJZIaiAidS57YJQnWCEAADmGnE5TyDeZqRkdMgvbMvU9Wyd6T05wI=3FF0BE2A".to_string();
        assert!(app.jira.has_valid_jira_token());
    }
}
