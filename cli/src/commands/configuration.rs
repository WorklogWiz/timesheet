use std::process::exit;

use worklog::config::JiraClientConfiguration;
use worklog::config::{self, AppConfiguration, ApplicationData};

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
                jira: settings.clone().into(),
                application_data: ApplicationData::default(),
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

impl From<UpdateConfiguration> for JiraClientConfiguration {
    fn from(val: UpdateConfiguration) -> Self {
        JiraClientConfiguration {
            user: val.user,
            token: val.token,
            url: val.url,
        }
    }
}
