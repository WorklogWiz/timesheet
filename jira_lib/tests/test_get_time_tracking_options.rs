use tokio;
use jira_lib;

#[tokio::test]
async fn test_get_time_tracking_options() {
    let jira_client = jira_lib::create_jira_client();
    let options = jira_client.get_time_tracking_options().await;
    assert_eq!(options.unwrap().defaultUnit, "hour".to_string());
}