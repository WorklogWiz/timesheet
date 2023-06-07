use std::fs::File;
use std::io;
use std::io::{ Read, Write};
use std::path::{Path, PathBuf};
use directories;
use directories::ProjectDirs;
use log::debug;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct ApplicationConfig {
    pub jira: Jira,
    pub dbms: WorklogDBMS,
}

impl Default for ApplicationConfig {
    fn default() -> Self {
        ApplicationConfig {
            jira: Default::default(),
            dbms: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct WorklogDBMS {
    // host=.... user=.... password=.... (note space as delimiteder beteween key/values
    pub connect: String,    // Connect string
}

impl Default for WorklogDBMS {
    fn default() -> Self {
        WorklogDBMS { connect: "host=postgres.testenv.autostoresystem.com user=postgres password=***************".to_string() }
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
        Jira { jira_url: "https://autostore.atlassian.net/rest/api/latest".to_string(), user: "user.name@autostoresystem.com".to_string(), token: "< your secrete Jira token goes here>".to_string() }
    }
}

pub fn config_file_name() -> PathBuf {
    let project_dirs = ProjectDirs::from("com", "autostore", "jira_worklog").expect("Unable to determine the name of the configuration file");
    let p = project_dirs.preference_dir();
    PathBuf::from(p)
}

pub fn load_configuration() -> Result<ApplicationConfig, io::Error> {
    read_configuration(config_file_name().as_path())
}

pub fn save_configuration(application_config: ApplicationConfig) {
    create_configuration_file(&application_config, &config_file_name())
}

pub fn create_and_save_sample_configuration() -> Result<ApplicationConfig, io::Error> {
    let application_config = ApplicationConfig::default();
    create_configuration_file(&application_config, &config_file_name());
    Ok(application_config)
}

pub fn load_or_create_configuration() -> Result<ApplicationConfig, io::Error> {
    match is_configuration_file_available() {
        None => {
             create_and_save_sample_configuration()
        }
        Some(app_config) => Ok(app_config)
    }
}

pub fn is_configuration_file_available() -> Option<ApplicationConfig> {
    let p = config_file_name();
    if p.exists() && p.is_file() {
        match load_configuration() {
            Ok(app_config) => Some(app_config),
            Err(e) => { panic!("Unable to load the configuration file from {}, reson: {}", config_file_name().to_string_lossy(), e) }
        }
    } else {
        Option::None
    }
}

fn read_configuration(path: &Path) -> Result<ApplicationConfig, io::Error> {
    let mut file = match File::open(path) {
        Ok(f) => f,
        Err(e) => return Err(e)
    };

    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    let config: ApplicationConfig = toml::from_str(&contents).unwrap();
    Ok(config)
}


fn create_configuration_file(application_config: &ApplicationConfig, path: &PathBuf) {
    let directory = path.parent().unwrap();
    match directory.try_exists() {
        Ok(_) => {}
        Err(_) => {
            debug!("Some parts of {} does not exist", directory.to_string_lossy());
            match std::fs::create_dir_all(directory) {
                Ok(_) => {
                    debug!("Created path {}", directory.to_string_lossy())
                }
                Err(e) => { panic!("Unable to create path {}, {}", directory.to_string_lossy(), e) }
            }
        }
    }
    let mut file = match File::create(path) {
        Ok(f) => f,
        Err(_) => panic!("Unable to create file named '{}'", path.to_string_lossy()),
    };
    let toml = application_config_to_string(application_config);

    match file.write_all(toml.as_bytes()) {
        Ok(_) => {}
        Err(e) => panic!("Unable to write configuration to TOML file: {}", e)
    };
}

pub fn application_config_to_string(application_config: &ApplicationConfig) -> String {
    let toml = match toml::to_string::<ApplicationConfig>(application_config) {
        Ok(s) => s,
        Err(e) => panic!("Unable to transform application config {:?} structure into Toml: {}", application_config, e),
    };
    toml
}

pub fn write_configuration(application_config: &ApplicationConfig, path: &PathBuf) {
    debug!("Writing configuration to {}", &path.to_string_lossy());

    let mut file = match File::create(path) {
        Ok(f) => f,
        Err(_) => panic!("Unable to create file named '{}'", path.to_string_lossy()),
    };
    let toml = match toml::to_string::<ApplicationConfig>(application_config) {
        Ok(s) => s,
        Err(e) => panic!("Unable to transform application config {:?} structure into Toml: {}", application_config, e),
    };

    debug!("Configuration is: {:?}", &toml);

    match file.write_all(toml.as_bytes()) {
        Ok(_) => {}
        Err(e) => panic!("Unable to write configuration to TOML file: {}", e)
    };
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn test_load_configuration() {
        let config_result = load_configuration();
        assert!(config_result.is_ok(), "Unable to load {}", config_file_name().to_string_lossy());

        let config = config_result.unwrap();
        println!("Config: {:?}", config);
    }

    #[test]
    fn test_write_and_read_toml_file() {
        let config_file_path = config_file_name().to_owned();
        let file_name = config_file_path.file_name().unwrap();

        let tmp_config_file = std::env::temp_dir().join(file_name);

        let application_config = ApplicationConfig::default();

        create_configuration_file(&application_config, &tmp_config_file);
        if let Ok(result) = read_configuration(&tmp_config_file) {
            assert_eq!(&application_config, &result);
        } else {
            panic!("Unable to read the TOML configuration back from disk");
        }
    }
}
