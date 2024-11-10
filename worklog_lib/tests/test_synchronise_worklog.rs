use anyhow::{anyhow, Context, Result};
use chrono::{Duration, Local};
use common::config;
use common::date;
use common::journal::Journal;
use jira_lib::{JiraClient, Worklog};
use journal_sql::JournalSqlite;
use log::{debug, Level};
use std::process::exit;
use worklog_lib::{ApplicationProductionRuntime, ApplicationRuntime};

fn test_production_runtime() {
    let runtime = ApplicationProductionRuntime::new();
}

#[tokio::test]
async fn test_sync() -> anyhow::Result<()> {
    common::configure_logging(Level::Debug);
    let app_config = config::load().map_err(|e| anyhow!("Unable to load configuration: {}", e))?;
    let dbms_file_name = config::tmp_local_worklog_dbms_file_name()?;
    let journal = JournalSqlite::new(&dbms_file_name)?;
    let jira_client = JiraClient::new(
        &app_config.jira.jira_url,
        &app_config.jira.user,
        &app_config.jira.token,
    )?;

    let mut keys = journal.find_unique_keys()?;
    debug!("Found these keys in the local DBMS {:?}", keys);
    if keys.len() == 0 {
        keys.push("TIME-147".to_string())
    }
    let now = Local::now();
    let start = now - Duration::days(30);

    let mut results: Vec<Worklog> = vec![];
    for key in keys {
        debug!("Retrieving worklogs for issue {} from {}", key, start);

        let mut worklogs = jira_client
            .get_worklogs_for_current_user(&key, Some(start))
            .await
            .map_err(|e| anyhow!("Unable to retrieve worklogs for {} : {}", key, e))?;
        results.append(&mut worklogs);
    }

    Ok(())
}
