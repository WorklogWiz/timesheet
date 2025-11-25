use jira::builder::DEFAULT_API_VERSION;
use jira::{Credentials, Jira};
use mockito::Server;

#[tokio::test]
async fn test_get_current_user_info() -> Result<(), Box<dyn std::error::Error>> {
    let mut server = Server::new_async().await;
    let url = server.url();

    let _m = server
        .mock(
            "GET",
            format!("/rest/api/{DEFAULT_API_VERSION}/myself").as_str(),
        )
        .with_status(200)
        .with_body(
            r#"{
            "self": "https://example.atlassian.net/rest/api/2/user?accountId=12345",
            "accountId": "12345abcde",
            "emailAddress": "user@example.com",
            "displayName": "Test User",
            "timeZone": "America/New_York"
        }"#,
        )
        .create_async()
        .await;

    let jira_client = Jira::new(
        url,
        Credentials::Basic("user@example.com".to_string(), "token".to_string()),
    )?;

    let current_user = jira_client.get_current_user().await?;
    assert!(!current_user.account_id.is_empty());
    assert_eq!(current_user.account_id, "12345abcde");
    assert!(!current_user.display_name.is_empty());
    assert_eq!(current_user.display_name, "Test User");
    assert_eq!(current_user.email_address, "user@example.com");

    Ok(())
}

#[tokio::test]
async fn test_get_time_tracking_options() -> Result<(), Box<dyn std::error::Error>> {
    let mut server = Server::new_async().await;
    let url = server.url();

    let _m = server
        .mock(
            "GET",
            format!("/rest/api/{DEFAULT_API_VERSION}/configuration").as_str(),
        )
        .with_status(200)
        .with_body(
            r#"{
            "votingEnabled": true,
            "watchingEnabled": true,
            "unassignedIssuesAllowed": true,
            "subTasksEnabled": true,
            "issueLinkingEnabled": true,
            "timeTrackingEnabled": true,
            "attachmentsEnabled": true,
            "timeTrackingConfiguration": {
                "defaultUnit": "hour",
                "timeFormat": "pretty",
                "workingHoursPerDay": 8.0,
                "workingDaysPerWeek": 5.0
            }
        }"#,
        )
        .create_async()
        .await;

    let jira_client = Jira::new(
        url,
        Credentials::Basic("user@example.com".to_string(), "token".to_string()),
    )?;

    let options = jira_client.get_time_tracking_options().await?;
    assert_eq!(options.defaultUnit, "hour");

    Ok(())
}
