/* TODO: Fix these tests using mockall
use chrono::{Days, Local, Utc};
use std::string::ToString;


#[ignore]
#[tokio::test] // Requires a valid user token in configuration
async fn test_get_time_tracking_options() {
    let jira_client = jira_lib::create_jira_client();
    let current_user = jira_client.get_current_user().await;
    assert!(!current_user.account_id.is_empty());
}

const TEST_ISSUE: &str = "TIME-147";

#[ignore]
#[tokio::test] // Requires a valid user token in configuration
async fn test_get_worklog_entries_for_current_user() {
    let jira_client = jira_lib::create_jira_client();

    let _current_user = jira_client.get_current_user().await;

    let utc = Utc::now();
    println!("UTC  : {:-30} {}", utc, utc.timestamp());
    let local = Local::now();
    println!("Local: {:-30} {}", local, local.timestamp());

    let r = jira_client
        .insert_worklog(TEST_ISSUE, Local::now(), 27000, "Rubbish comment")
        .await;
    println!("Received a response: {r:?}");
    assert!(r.is_ok(), "Insertion failed {r:?}");

    println!("Inserted ok");

    let result = jira_client
        .get_worklogs_for_current_user(TEST_ISSUE, Local::now().checked_sub_days(Days::new(1)))
        .await;
    assert!(result.as_deref().is_ok(), "HTTP request failed {result:?}");
    let result = result.unwrap();
    assert!(!result.is_empty());
    assert!(!result[0].id.is_empty());

    let r = r.unwrap();
    let _r = jira_client
        .delete_worklog(TEST_ISSUE.to_string(), r.id)
        .await;
}

#[ignore]
#[tokio::test]
async fn test_get_time_tracking_options() {
    let jira_client = jira_lib::create_jira_client();
    let options = jira_client.get_time_tracking_options().await;
    assert_eq!(options.unwrap().defaultUnit, "hour".to_string());
}
*/
