use std::process::exit;

use worklog::config;

use crate::cli::Configuration;

fn list_and_exit() {
    println!(
        "Configuration file {}:\n",
        config::configuration_file().to_string_lossy()
    );

    match config::load() {
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

pub fn execute(config: Configuration) {
    match config {
        // List current configuration
        Configuration {
            list: true,
            remove: false,
            ..
        } => {
            list_and_exit();
        }
        // Add new values to the configuration
        Configuration {
            user,
            token,
            jira_url,
            tracking_project,
            list: false,
            remove: false,
        } => {
            let mut app_config = match config::load_or_create() {
                Ok(ac) => ac,
                Err(e) => {
                    eprintln!(
                        "ERROR: Unable to load or create configuration file {}, reason:{}",
                        config::configuration_file().to_string_lossy(),
                        e
                    );
                    exit(4);
                }
            };
            if let Some(user) = user {
                app_config.jira.user = user.to_string();
            }
            if let Some(token) = token {
                app_config.jira.token = token.to_string();
            }
            if let Some(jira_url) = jira_url {
                app_config.jira.jira_url = jira_url.to_string();
            }
            if let Some(tracking_project) = tracking_project {
                app_config.tracking_project = tracking_project.to_string();
            }
            config::save(&app_config).expect("Unable to save the application config");
            println!(
                "Configuration saved to {}",
                config::configuration_file().to_string_lossy()
            );
            exit(0);
        }
        Configuration { remove: true, .. } => match config::remove() {
            Ok(()) => {
                println!(
                    "Configuration file {} removed",
                    config::configuration_file().to_string_lossy()
                );
            }
            Err(e) => {
                println!(
                    "ERROR:Unable to remove configuration file {} : {}",
                    config::configuration_file().to_string_lossy(),
                    e
                );
            }
        },
    }
}
