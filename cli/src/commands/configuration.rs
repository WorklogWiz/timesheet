use std::process::exit;

use worklog_config::config::{
    self, AppConfiguration, IssueTrackerConfiguration, StorageConfiguration,
};

use crate::cli::{ConfigCommand, UpdateConfiguration};

fn list_and_exit() {
    println!(
        "Configuration file {}:\n",
        config::configuration_file().to_string_lossy()
    );

    match config::load_with_keychain_lookup() {
        Ok(config) => {
            let toml_as_string = config::application_config_to_string(&config).unwrap();
            println!("{toml_as_string}");
        }
        Err(_) => {
            println!("Config file does not exist or is empty. Use --token and --user to create it");
        }
    }
    exit(0);
}

#[allow(clippy::enum_glob_use)]
pub fn execute(config: ConfigCommand) {
    use ConfigCommand::*;
    match config {
        List => {
            list_and_exit();
        }
        // Add new values to the configuration
        Update(settings) => {
            let app_config = AppConfiguration {
                storage: StorageConfiguration::default(),
                issue_tracker: settings.clone().into(),
                jira: None,
                application_data: None,
            };

            config::save(&app_config).expect("Unable to save the application config");
            println!(
                "Configuration saved to {}",
                config::configuration_file().to_string_lossy()
            );
            exit(0);
        }
        Remove => match config::remove() {
            Ok(()) => {
                println!(
                    "Configuration file {} removed",
                    config::configuration_file().to_string_lossy()
                );
            }
            Err(e) => {
                println!(
                    "ERROR:Unable to remove configuration file {} : {e}",
                    config::configuration_file().to_string_lossy(),
                );
            }
        },
    }
}

impl From<UpdateConfiguration> for IssueTrackerConfiguration {
    fn from(val: UpdateConfiguration) -> Self {
        let url = val
            .url
            .trim_start_matches("https://")
            .trim_start_matches("http://");

        let mut config = std::collections::HashMap::new();
        config.insert("username".to_string(), val.user);
        config.insert("token".to_string(), val.token);

        IssueTrackerConfiguration {
            url: format!("jira://{url}"),
            config,
        }
    }
}
