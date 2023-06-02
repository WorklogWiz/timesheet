use chrono::{Days, Local, Utc};
use tokio;
use jira_lib;

#[tokio::test]
async fn test_get_time_tracking_options() {
    let jira_client = jira_lib::create_jira_client();
    let current_user = jira_client.get_current_user().await;
    assert!(!current_user.account_id.is_empty());
}


#[tokio::test]
async fn test_get_worklog_entries_for_current_user(){
    let jira_client = jira_lib::create_jira_client();


    let utc = Utc::now();
    println!("UTC  : {:-30} {}", utc,utc.timestamp());
    let local = Local::now();
    println!("Local: {:-30} {}", local, local.timestamp());

    let r = jira_client.insert_worklog( "TIME-94", Local::now(), 27000, "Rubbish comment").await;
    println!("Received a respone: {:?}", r);

    let result = jira_client.get_worklogs_for_current_user("TIME-94",Local::now().checked_sub_days(Days::new(1))).await;
    assert!(!result.is_empty());
    assert!(!result[0].id.is_empty());

    // let r = jira_client.delete_worklog(r.id).await;

}