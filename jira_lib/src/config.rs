use std::fs::File;
use std::io;
use std::io::Read;
use std::path::{ PathBuf};
use directories;
use directories::ProjectDirs;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct ApplicationConfig {
    pub jira: Jira,
    pub dbms: WorklogDBMS,
}

#[derive(Deserialize, Debug)]
pub struct WorklogDBMS {
    // host=.... user=.... password=.... (note space as delimiteder beteween key/values
    pub connect: String,    // Connect string
}

#[derive(Deserialize, Debug)]
pub struct Jira {
    pub jira_url: String,
    pub user: String,
    pub token: String,
}

pub fn config_file_name() -> PathBuf {
    let project_dirs = ProjectDirs::from("com", "autostore", "jira_worklog.toml").expect("Unable to determine the name of the configuration file");
    let p = project_dirs.preference_dir();
    PathBuf::from(p)
}

pub fn load_configuration() -> Result<ApplicationConfig, io::Error>{
    let mut file = match File::open(config_file_name()) {
        Ok(f) => f,
        Err(e) => panic!("Unable to load config file {}", config_file_name().to_string_lossy()),
    };
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
