use std::env;

use chrono::{Months, NaiveDateTime, NaiveTime};
use jira::{Credentials, Jira};

#[tokio::main]
async fn main() {
    std::env::set_var("RUST_LOG", "debug");
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
        for (i, project) in projects.iter().enumerate() {
            println!(
                "{:>3} {} {} {}, private={}",
                i, project.id, project.key, project.name, project.is_private
            );
        }
        let worklogs = client
            .get_worklogs_for("A3SRS-1".to_string(), midnight_a_month_ago_in())
            .await;
        println!("{:?}", &worklogs);

        let results = client
            .get_worklogs_for_current_user("time-147", Option::None)
            .await;
        if let Ok(worklogs) = results {
            for worklog in worklogs {
                println!("{} {} {}", worklog.id, worklog.started, worklog.timeSpent);
            }
        } else {
            println!("Unable to retrieve your worklogs for TIME-147");
        }
    } else {
        panic!("Missing env var JIRA_HOST, JIRA_USER or JIRA_TOKEN")
    }
}

fn midnight_a_month_ago_in() -> NaiveDateTime {
    let today = chrono::offset::Local::now();
    let a_month_ago = today.checked_sub_months(Months::new(1)).unwrap();
    NaiveDateTime::new(
        a_month_ago.date_naive(),
        NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
    )
}
