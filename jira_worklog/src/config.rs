use std::fs::File;
use std::io;
use std::io::Read;
use std::path::{Path, PathBuf};
use directories;
use directories::ProjectDirs;
use tokio::io::AsyncReadExt;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct ApplicationConfig {
    jira: Jira,
}

#[derive(Deserialize, Debug)]
pub struct Jira {
    jira_url: String,
    user: String,
    token: String,
}

pub fn config_file_name() -> PathBuf {
    let project_dirs = ProjectDirs::from("com", "autostore", "jira_worklog.toml").expect("Unable to determine the name of the configuration file");
    let p = project_dirs.preference_dir();
    PathBuf::from(p)
}

pub fn load_configuration() -> Result<ApplicationConfig, io::Error>{
    let mut file = File::open(config_file_name())?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    let config: ApplicationConfig = toml::from_str(&contents).unwrap();
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn test_load_configuration(){
        let config_result = load_configuration();
        assert!(config_result.is_ok(),"Unable to load {}", config_file_name().to_string_lossy());

        let config = config_result.unwrap();
        println!("Config: {:?}", config);
    }
}
