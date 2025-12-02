use jira::builder::DEFAULT_API_VERSION;
use jira::models::core::IssueKey;
use jira::{Credentials, Jira};
use mockito::Server;

#[tokio::test]
async fn test_get_issue_summary() -> Result<(), Box<dyn std::error::Error>> {
    let mut server = Server::new_async().await;
    let url = server.url();

    // Mock GET /issue/{key} endpoint
    let _m = server
        .mock(
            "GET",
            format!(
                "/rest/api/{DEFAULT_API_VERSION}/issue/TEST-123?fields=id,key,summary,components"
            )
            .as_str(),
        )
        .with_status(200)
        .with_body(
            r#"{
            "id": "12345",
            "key": "TEST-123",
            "fields": {
                "summary": "Test issue summary",
                "components": [
                    {"id": "comp-1", "name": "Backend"}
                ]
            }
        }"#,
        )
        .create_async()
        .await;

    let jira_client = Jira::new(
        url,
        Credentials::Basic("user@example.com".to_string(), "token".to_string()),
    )?;

    let issue = jira_client
        .get_issue_summary(&IssueKey::new("TEST-123"))
        .await?;

    assert_eq!(issue.key.value, "TEST-123");
    assert_eq!(issue.id, "12345");
    assert_eq!(issue.fields.summary, "Test issue summary");
    assert_eq!(issue.fields.components.len(), 1);
    assert_eq!(issue.fields.components[0].name, "Backend");

    Ok(())
}

#[tokio::test]
async fn test_get_issue_summary_not_found() -> Result<(), Box<dyn std::error::Error>> {
    let mut server = Server::new_async().await;
    let url = server.url();

    // Mock 404 response
    let _m = server
        .mock(
            "GET",
            format!("/rest/api/{DEFAULT_API_VERSION}/issue/NOTFOUND-999?fields=id,key,summary,components")
            .as_str(),
        )
        .with_status(404)
        .with_body(
            r#"{
            "errorMessages": ["Issue does not exist or you do not have permission to see it."],
            "errors": {}
        }"#,
        )
        .create_async()
        .await;

    let jira_client = Jira::new(
        url,
        Credentials::Basic("user@example.com".to_string(), "token".to_string()),
    )?;

    let result = jira_client
        .get_issue_summary(&IssueKey::new("NOTFOUND-999"))
        .await;

    assert!(result.is_err());
    match result {
        Err(jira::JiraError::NotFound(msg)) => {
            assert_eq!(msg, "NOTFOUND-999");
        }
        _ => panic!("Expected NotFound error"),
    }

    Ok(())
}
