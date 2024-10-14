use directories;
use directories::ProjectDirs;
use log::debug;
use serde::{Deserialize, Serialize};
use std::fs::{create_dir_all, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::exit;
use std::{fs, io};

#[derive(Serialize, Deserialize, Debug, PartialEq, Default)]
pub struct Application {
    pub jira: Jira,
    pub dbms: WorklogDBMS,
    /// This will ensure that the filename is created, even if the Toml file
    /// is an old version, which does not have an `application_data` section
    #[serde(default = "default_application_data_section")]
    pub application_data: ApplicationData,
}

/// Holds the configuration for the `application_data` section of the Toml file
#[derive(Serialize, Deserialize, Debug, PartialEq)]
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

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct WorklogDBMS {
    // host=.... user=.... password=.... (note space as delimiter between key/values
    pub connect: String, // Connect string
}

impl Default for WorklogDBMS {
    fn default() -> Self {
        WorklogDBMS {
            connect:
                "host=postgres.testenv.autostoresystem.com user=postgres password=***************"
                    .to_string(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
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
            token: "< your secrete Jira token goes here>".to_string(),
        }
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
#[allow(clippy::missing_errors_doc)]
pub fn load_configuration() -> Result<Application, io::Error> {
    read_configuration(file_name().as_path())
}

#[allow(clippy::missing_errors_doc)]
pub fn save_configuration(application_config: &Application) -> std::io::Result<()> {
    create_configuration_file(application_config, &file_name())
}

#[allow(clippy::missing_errors_doc)]
pub fn remove_configuration() -> std::io::Result<()> {
    fs::remove_file(file_name().as_path())
}

#[allow(clippy::missing_errors_doc)]
pub fn create_and_save_sample_configuration() -> Result<Application, io::Error> {
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
pub fn is_configuration_file_available() -> Option<Application> {
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
        Option::None
    }
}

fn read_configuration(path: &Path) -> Result<Application, io::Error> {
    let mut file = match File::open(path) {
        Ok(f) => f,
        Err(e) => {
            debug!("Unable to open {}, cause: {}", path.to_string_lossy(),e);
            return Err(e)
        },
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

#[allow(clippy::manual_assert)]
fn create_configuration_file(application_config: &Application, path: &PathBuf) -> Result<(), io::Error> {
    debug!("create_configuration_file({}) :- entering ..", path.to_string_lossy());

    let directory = path.parent().unwrap();
    if directory.try_exists()? {
        debug!("Path {} exists", directory.to_string_lossy());
    } else {
        std::fs::create_dir_all(directory)?;
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

    let mut file = File::create(path)?;
    let toml = application_config_to_string(application_config);

    file.write_all(toml.as_bytes())?;

    Ok(())
}

#[allow(clippy::missing_panics_doc)]
#[must_use]
pub fn application_config_to_string(application_config: &Application) -> String {
    match toml::to_string::<Application>(application_config) {
        Ok(s) => s,
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
}
