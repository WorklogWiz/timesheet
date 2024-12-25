use std::env;

use jira::{Credentials, Jira};

#[tokio::main]
async fn main() {
    env_logger::init();

    if let (Ok(host), Ok(user), Ok(token)) = (
        env::var("JIRA_HOST"),
        env::var("JIRA_USER"),
        env::var("JIRA_TOKEN"),
    ) {
        let client = Jira::new(&host, Credentials::Basic(user, token))
            .expect("Error initializing jira client");

        let projects = client
            .get_projects(vec![])
            .await
            .expect("Failed to get projects");

        println!("Found {} projects", projects.len());
        println!(
            "{:>3} {:6} {:6} {:40} {}",
            "No", "ID", "KEY", "NAME", "PRIVATE"
        );
        for (i, project) in projects.iter().enumerate() {
            println!(
                "{:>3} {:6} {:6} {:40}, {}",
                i, project.id, project.key, project.name, project.is_private
            );
        }
    } else {
        panic!("Missing env var JIRA_HOST, JIRA_USER or JIRA_TOKEN")
    }
}
