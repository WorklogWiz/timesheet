use crate::error::WorklogError;
use anyhow::Result;
use directories;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

#[cfg(target_os = "macos")]
use log::debug;

#[cfg(target_os = "macos")]
pub const KEYCHAIN_SERVICE_NAME: &str = "com.norn.timesheet.jira";

/// Application configuration struct
/// Holds the data we need to connect to Jira, write to the local journal and so on
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct AppConfiguration {
    /// Holds the URL to the Jira instance we are running again.
    pub jira: JiraClientConfiguration,

    /// This will ensure that the filename is created, even if the Toml file
    /// is an old version, which does not have an `application_data` section
    #[serde(default = "default_application_data")]
    pub application_data: ApplicationData,
}

/// Holds the configuration for the `application_data` section of the Toml file
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct ApplicationData {
    /// The path to the local worklog data store
    pub local_worklog: String,
}

impl Default for ApplicationData {
    fn default() -> Self {
        ApplicationData {
            local_worklog: worklog_file().to_string_lossy().to_string(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct JiraClientConfiguration {
    pub url: String,
    pub user: String,
    pub token: String,
}

impl JiraClientConfiguration {
    /// Does the token look like a valid Jira Security token?
    #[must_use]
    pub fn has_valid_jira_token(&self) -> bool {
        !(self.token.contains("secret") || self.token == JIRA_TOKEN_STORED_IN_MACOS_KEYCHAIN)
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

/// Loads and returns the application configuration, with platform-specific keychain integration, c
/// currently only for macOS.
///
/// On macOS, this function performs additional keychain-related operations:
/// - If a valid Jira token exists in the configuration file but not in the keychain,
///   it migrates the token to the keychain
/// - Retrieves the Jira token from the keychain and merges it into the configuration
///
/// On other platforms, this behaves the same as `load_no_keychain_lookup()`.
///
/// # Errors
///
/// Returns `WorklogError` if:
/// - The configuration file cannot be read or parsed
/// - The configuration file cannot be created during token migration on macOS
///
/// # Example
///
/// ```no_run
/// use worklog::config;
///
/// let app_config = config::load_with_keychain_lookup()
///     .expect("Failed to load configuration");
/// println!("Jira URL: {}", app_config.jira.url);
/// ```
#[allow(unused_mut)]
pub fn load_with_keychain_lookup() -> Result<AppConfiguration, WorklogError> {
    // Loads the plain configuration file without a keychain lookup
    let (config_path, mut app_config) = load_no_keychain_lookup()?;

    #[cfg(target_os = "macos")]
    if cfg!(target_os = "macos") {
        // If the loaded configuration file holds a valid Jira token, migrate it to
        // the macOS Key Chain
        if app_config.jira.has_valid_jira_token()
            && secure_credentials::macos::get_secure_token(
                KEYCHAIN_SERVICE_NAME,
                &app_config.jira.user,
            )
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

/// Loads the application configuration from the configuration file without performing any keychain lookups.
///
/// This function reads and parses the TOML configuration file from the default configuration path.
/// It does not attempt to retrieve or store any credentials in the system keychain, making it suitable
/// for all platforms and testing scenarios.
///
/// # Returns
///
/// Returns a tuple containing:
/// - The path to the configuration file (`PathBuf`)
/// - The parsed application configuration (`AppConfiguration`)
///
/// # Errors
///
/// Returns `WorklogError` if:
/// - The configuration file cannot be found
/// - The file cannot be read due to permissions or I/O errors
/// - The TOML content cannot be parsed into the `AppConfiguration` structure
///
/// # Example
///
/// ```no_run
/// use worklog::config;
///
/// let (config_path, app_config) = config::load_no_keychain_lookup()
///     .expect("Failed to load configuration");
/// println!("Configuration loaded from: {}", config_path.display());
/// ```
pub fn load_no_keychain_lookup() -> Result<(PathBuf, AppConfiguration), WorklogError> {
    let config_path = configuration_file();

    let app_config = read_data(&config_path)?;
    Ok((config_path, app_config))
}

#[allow(clippy::missing_errors_doc)]
pub fn save(cfg: &AppConfiguration) -> Result<()> {
    create_configuration_file(cfg, &configuration_file())
}

#[allow(clippy::missing_errors_doc)]
pub fn remove() -> io::Result<()> {
    fs::remove_file(configuration_file().as_path())
}

#[allow(clippy::missing_errors_doc)]
pub fn application_config_to_string(cfg: &AppConfiguration) -> Result<String> {
    Ok(toml::to_string::<AppConfiguration>(cfg)?)
}

fn default_application_data() -> ApplicationData {
    ApplicationData::default()
}

fn project_dirs() -> ProjectDirs {
    ProjectDirs::from("com", "norn", "timesheet")
        .expect("Unable to determine the name of the 'project_dirs' directory name")
}

/// Reads and parses an `AppConfiguration` from a TOML configuration file.
///
/// This function reads the contents of the specified TOML file and attempts to parse
/// it into an `AppConfiguration` struct.
///
/// # Arguments
///
/// * `path` - Path to the TOML configuration file to read
///
/// # Returns
///
/// Returns `Result<AppConfiguration, WorklogError>` where:
/// - `Ok(AppConfiguration)` - Successfully parsed configuration
/// - `Err(WorklogError)` - If the file cannot be read or parsed
///
/// # Errors
///
/// Returns `WorklogError` if:
/// - The file cannot be opened or read (`WorklogError::ApplicationConfig`)
/// - The TOML content cannot be parsed (`WorklogError::TomlParse`)
///
/// # Example
///
/// ```no_run
/// use std::path::Path;
/// use worklog::config::read_data;
///
/// let config = read_data(Path::new("config.toml"))
///     .expect("Failed to read configuration");
/// ```
pub fn read_data(path: &Path) -> Result<AppConfiguration, WorklogError> {
    let mut file = File::open(path).map_err(|source| WorklogError::ApplicationConfig {
        path: path.to_string_lossy().into(),
        source,
    })?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .map_err(|source| WorklogError::ApplicationConfig {
            path: path.to_string_lossy().into(),
            source,
        })?;
    toml::from_str::<AppConfiguration>(&contents).map_err(|source| WorklogError::TomlParse {
        path: path.into(),
        source: Box::new(source),
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
/// security add-generic-password -s com.norn.timesheet \
///   -a your-emailk@whereever.com -w secure_token_goes_here
/// `
#[cfg(target_os = "macos")]
fn merge_jira_token_from_keychain(config: &mut AppConfiguration) {
    use log::warn;

    debug!("MacOS: retrieving the Jira access token from the keychain ...");
    match secure_credentials::macos::get_secure_token(KEYCHAIN_SERVICE_NAME, &config.jira.user) {
        Ok(token) => {
            debug!("Found Jira access token in keychain and injected it");
            config.jira.token = token;
        }
        Err(err) => {
            warn!(
                "No Jira Access Token in keychain for {} and {}",
                KEYCHAIN_SERVICE_NAME, &config.jira.user
            );
            warn!("ERROR: {err}");
            eprintln!(
                "No Jira Access Token in keychain for {} and {}",
                KEYCHAIN_SERVICE_NAME, &config.jira.user
            );
            eprintln!("If this is the first time your using the tool, this warning can be ignored");
        }
    }
}

const JIRA_TOKEN_STORED_IN_MACOS_KEYCHAIN: &str = "*** stored in macos keychain ***";

#[cfg(target_os = "macos")]
fn migrate_jira_token_into_keychain(app_config: &mut AppConfiguration) {
    match secure_credentials::macos::store_secure_token(
        KEYCHAIN_SERVICE_NAME,
        &app_config.jira.user,
        &app_config.jira.token,
    ) {
        Ok(()) => {
            debug!(
                "Jira access token stored into the Keychain under {} and {}",
                KEYCHAIN_SERVICE_NAME, app_config.jira.user
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
    fn toml_parsing() {
        let toml_str = r#"
        [jira]
        url = "http"
        user = "steinar"
        token = "rubbish"

        [application_data]
        local_worklog = "worklog.db"
        "#;

        let app_config: AppConfiguration = toml::from_str(toml_str).unwrap();
        assert_eq!(app_config.application_data.local_worklog, "worklog.db");
    }

    /// Verifies that the `journal_data_file_name` is populated with a reasonable default even if it
    /// does not exist in the configuration file on disk
    #[test]
    fn test_toml_parsing_with_defaults_generated() {
        let toml_str = r#"
        [jira]
        url = "http"
        user = "steinar"
        token = "rubbish"
        "#;

        let app_config: AppConfiguration = toml::from_str(toml_str).unwrap();
        assert_eq!(
            app_config.application_data.local_worklog,
            worklog_file().to_string_lossy()
        );
    }

    #[ignore = "Cannot access the keychain from a non-interactive test"]
    #[test]
    fn test_write_and_read_toml_file() -> Result<()> {
        let tmp_config_file = std::env::temp_dir().join("test-config.toml");

        let cfg = generate_config_for_test();

        create_configuration_file(&cfg, &tmp_config_file)?;
        if let Ok(result) = read_data(&tmp_config_file) {
            // Don't compare the jira.token field as this may vary depending on operating system
            assert!(
                cfg.jira.url == result.jira.url
                    && cfg.jira.user == result.jira.user
                    && cfg.application_data == result.application_data
            );
        } else {
            panic!("Unable to read the TOML configuration back from disk");
        }

        Ok(())
    }

    fn generate_config_for_test() -> AppConfiguration {
        AppConfiguration {
            jira: JiraClientConfiguration {
                url: "http".to_string(),
                user: "steinar".to_string(),
                token: "not_a_token".to_string(),
            },
            application_data: ApplicationData {
                local_worklog: "worklog.db".to_string(),
            },
        }
    }
}
