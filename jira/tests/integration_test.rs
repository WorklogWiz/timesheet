mod test_helpers;

use std::string::ToString;
use test_helpers::jira_client;

#[tokio::test] // Requires a valid user token in configuration
async fn test_get_current_user_info() -> Result<(), Box<dyn std::error::Error>> {
    let jira_client = jira_client::create();
    let current_user = jira_client.get_current_user().await?;
    assert!(!current_user.account_id.is_empty());
    assert!(!current_user.display_name.is_empty());
    Ok(())
}

#[tokio::test]
async fn test_get_time_tracking_options() -> Result<(), Box<dyn std::error::Error>> {
    let jira_client = jira_client::create();
    let options = jira_client.get_time_tracking_options().await?;
    assert_eq!(options.defaultUnit, "hour".to_string());

    Ok(())
}
