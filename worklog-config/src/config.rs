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
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct AppConfiguration {
    #[serde(default)]
    pub storage: StorageConfiguration,

    #[serde(default)]
    pub issue_tracker: IssueTrackerConfiguration,

    // Legacy field for backward compatibility
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jira: Option<LegacyJiraConfiguration>,

    // Legacy field for backward compatibility
    #[serde(skip_serializing_if = "Option::is_none")]
    pub application_data: Option<LegacyApplicationData>,
}

/// Storage configuration (provider-agnostic)
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct StorageConfiguration {
    pub url: String,
    #[serde(default)]
    pub options: std::collections::HashMap<String, String>,
}

impl Default for StorageConfiguration {
    fn default() -> Self {
        Self {
            url: format!("sqlite://{}", worklog_file().to_string_lossy()),
            options: std::collections::HashMap::new(),
        }
    }
}

/// Issue tracker configuration (provider-agnostic)
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct IssueTrackerConfiguration {
    pub url: String,
    #[serde(default)]
    pub config: std::collections::HashMap<String, String>,
}

impl Default for IssueTrackerConfiguration {
    fn default() -> Self {
        Self {
            url: "jira://localhost".to_string(),
            config: std::collections::HashMap::new(),
        }
    }
}

impl IssueTrackerConfiguration {
    /// Check if the tracker has a valid authentication token
    #[must_use]
    pub fn has_valid_token(&self) -> bool {
        self.config.get("token").is_some_and(|token| {
            !token.contains("secret") && token != JIRA_TOKEN_STORED_IN_MACOS_KEYCHAIN
        })
    }

    /// Get the username/account identifier for keychain lookups
    #[must_use]
    pub fn get_username(&self) -> Option<&str> {
        self.config.get("username").map(String::as_str)
    }

    /// Set the authentication token
    pub fn set_token(&mut self, token: String) {
        self.config.insert("token".to_string(), token);
    }

    /// Get the authentication token
    #[must_use]
    pub fn get_token(&self) -> Option<&str> {
        self.config.get("token").map(String::as_str)
    }
}

// Legacy types for backward compatibility (will be removed in future version)
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct LegacyJiraConfiguration {
    pub url: String,
    pub user: String,
    pub token: String,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct LegacyApplicationData {
    pub local_worklog: String,
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
/// use worklog_config::config;
///
/// let app_config = config::load_with_keychain_lookup()
///     .expect("Failed to load configuration");
/// println!("Issue Tracker URL: {}", app_config.issue_tracker.url);
/// ```
#[allow(unused_mut)]
pub fn load_with_keychain_lookup() -> Result<AppConfiguration, WorklogError> {
    // Loads the plain configuration file without a keychain lookup
    #[cfg_attr(not(target_os = "macos"), allow(unused_variables))]
    let (config_path, mut app_config) = load_no_keychain_lookup()?;

    // Migrate legacy configuration to new structure if needed
    migrate_legacy_config(&mut app_config);

    #[cfg(target_os = "macos")]
    if cfg!(target_os = "macos") {
        // If the loaded configuration file holds a valid token, migrate it to the macOS Keychain
        if app_config.issue_tracker.has_valid_token() {
            if let Some(username) = app_config.issue_tracker.get_username() {
                if crate::macos::get_secure_token(KEYCHAIN_SERVICE_NAME, username).is_err() {
                    migrate_token_into_keychain(&mut app_config);
                    create_configuration_file(&app_config, &config_path)
                        .map_err(|_| WorklogError::ConfigFileCreation { path: config_path })?;
                }
            }
        }

        // Retrieve the token from the Keychain and merge it into the configuration
        merge_token_from_keychain(&mut app_config);
    }
    Ok(app_config)
}

/// Migrate legacy `[jira]` and `[application_data]` config to new structure
fn migrate_legacy_config(config: &mut AppConfiguration) {
    // Migrate legacy [jira] section to [issue_tracker]
    if let Some(jira) = &config.jira {
        if config.issue_tracker.url == "jira://localhost" {
            // Only migrate if issue_tracker is still at default
            let url = jira
                .url
                .trim_start_matches("https://")
                .trim_start_matches("http://");
            config.issue_tracker.url = format!("jira://{url}");
            config
                .issue_tracker
                .config
                .insert("username".to_string(), jira.user.clone());
            config
                .issue_tracker
                .config
                .insert("token".to_string(), jira.token.clone());
        }
        config.jira = None; // Remove legacy field
    }

    // Migrate legacy [application_data] section to [storage]
    if let Some(app_data) = &config.application_data {
        if config.storage.url.contains("worklog.db")
            && app_data.local_worklog != worklog_file().to_string_lossy()
        {
            // Only migrate if storage is at default and legacy has different path
            config.storage.url = format!("sqlite://{}", app_data.local_worklog);
        }
        config.application_data = None; // Remove legacy field
    }
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
/// use worklog_config::config;
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
/// use worklog_config::config::read_data;
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
        debug!("MacOS: Moving security token into the keychain");
        migrate_token_into_keychain(&mut cfg_updated);
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
fn merge_token_from_keychain(config: &mut AppConfiguration) {
    use log::warn;

    if let Some(username) = config.issue_tracker.get_username() {
        debug!("MacOS: retrieving the issue tracker access token from the keychain ...");
        match crate::macos::get_secure_token(KEYCHAIN_SERVICE_NAME, username) {
            Ok(token) => {
                debug!("Found access token in keychain and injected it");
                config.issue_tracker.set_token(token);
            }
            Err(err) => {
                warn!("No Access Token in keychain for {KEYCHAIN_SERVICE_NAME} and {username}");
                warn!("ERROR: {err}");
                eprintln!("No Access Token in keychain for {KEYCHAIN_SERVICE_NAME} and {username}");
                eprintln!("If this is the first time using the tool, this warning can be ignored");
            }
        }
    }
}

const JIRA_TOKEN_STORED_IN_MACOS_KEYCHAIN: &str = "*** stored in macos keychain ***";

#[cfg(target_os = "macos")]
fn migrate_token_into_keychain(app_config: &mut AppConfiguration) {
    if let (Some(username), Some(token)) = (
        app_config.issue_tracker.get_username(),
        app_config.issue_tracker.get_token(),
    ) {
        match crate::macos::store_secure_token(KEYCHAIN_SERVICE_NAME, username, token) {
            Ok(()) => {
                debug!("Access token stored into the Keychain under {KEYCHAIN_SERVICE_NAME} and {username}");
                debug!("MacOS: Removing the security token from the config file");
            }
            #[allow(unused_variables)]
            Err(error) => {
                panic!("Unable to store the access token into the MacOS keychain {error}");
            }
        }
        // Replace token in config file with placeholder
        app_config
            .issue_tracker
            .set_token(JIRA_TOKEN_STORED_IN_MACOS_KEYCHAIN.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toml_parsing() {
        let toml_str = r#"
        [storage]
        url = "sqlite://worklog.db"

        [issue_tracker]
        url = "jira://example.com"

        [issue_tracker.config]
        username = "steinar"
        token = "rubbish"
        "#;

        let app_config: AppConfiguration = toml::from_str(toml_str).unwrap();
        assert_eq!(app_config.storage.url, "sqlite://worklog.db");
    }

    /// Verifies that the storage URL is populated with a reasonable default even if it
    /// does not exist in the configuration file on disk
    #[test]
    fn test_toml_parsing_with_defaults_generated() {
        let toml_str = r#"
        [issue_tracker]
        url = "jira://example.com"

        [issue_tracker.config]
        username = "steinar"
        token = "rubbish"
        "#;

        let app_config: AppConfiguration = toml::from_str(toml_str).unwrap();
        assert_eq!(
            app_config.storage.url,
            format!("sqlite://{}", worklog_file().to_string_lossy())
        );
    }

    #[ignore = "Cannot access the keychain from a non-interactive test"]
    #[test]
    fn test_write_and_read_toml_file() -> Result<()> {
        let tmp_config_file = std::env::temp_dir().join("test-config.toml");

        let cfg = generate_config_for_test();

        create_configuration_file(&cfg, &tmp_config_file)?;
        if let Ok(result) = read_data(&tmp_config_file) {
            // Don't compare the token field as this may vary depending on operating system
            assert!(
                cfg.issue_tracker.url == result.issue_tracker.url
                    && cfg.issue_tracker.get_username() == result.issue_tracker.get_username()
                    && cfg.storage == result.storage
            );
        } else {
            panic!("Unable to read the TOML configuration back from disk");
        }

        Ok(())
    }

    fn generate_config_for_test() -> AppConfiguration {
        let mut config = std::collections::HashMap::new();
        config.insert("username".to_string(), "steinar".to_string());
        config.insert("token".to_string(), "not_a_token".to_string());

        AppConfiguration {
            storage: StorageConfiguration {
                url: "sqlite://worklog.db".to_string(),
                options: std::collections::HashMap::new(),
            },
            issue_tracker: IssueTrackerConfiguration {
                url: "jira://example.com".to_string(),
                config,
            },
            jira: None,
            application_data: None,
        }
    }
}
