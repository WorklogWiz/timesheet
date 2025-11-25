use jira::builder::DEFAULT_API_VERSION;
use jira::{Credentials, Jira};
use mockito::Server;

#[tokio::test]
async fn test_create_issue() -> Result<(), Box<dyn std::error::Error>> {
    let mut server = Server::new_async().await;
    let url = server.url();

    // Mock POST /issue (create issue)
    let _m_create = server
        .mock(
            "POST",
            format!("/rest/api/{DEFAULT_API_VERSION}/issue").as_str(),
        )
        .with_status(201)
        .with_body(
            r#"{
            "id": "12345",
            "key": "TEST-123"
        }"#,
        )
        .create_async()
        .await;

    // Mock DELETE /issue/TEST-123 (delete issue)
    let _m_delete = server
        .mock(
            "DELETE",
            format!("/rest/api/{DEFAULT_API_VERSION}/issue/TEST-123").as_str(),
        )
        .with_status(204)
        .create_async()
        .await;

    let jira_client = Jira::new(
        url,
        Credentials::Basic("user@example.com".to_string(), "token".to_string()),
    )?;

    let new_issue = jira_client
        .create_issue(
            &jira::models::project::JiraProjectKey { key: "TEST" },
            "Test issue",
            Some("Test description".to_string()),
            vec![],
        )
        .await?;

    assert_eq!(new_issue.key.value, "TEST-123");
    assert_eq!(new_issue.id, "12345");

    // Delete the issue
    jira_client.delete_issue(&new_issue.key).await?;

    Ok(())
}

#[tokio::test]
async fn test_create_multiple_issues() -> Result<(), Box<dyn std::error::Error>> {
    let mut server = Server::new_async().await;
    let url = server.url();

    // Mock multiple create issue calls
    // Note: We'll create individual mocks for each call since mockito 1.x has limitations
    for i in 1..=10 {
        server
            .mock(
                "POST",
                format!("/rest/api/{DEFAULT_API_VERSION}/issue").as_str(),
            )
            .with_status(201)
            .with_body(format!(
                r#"{{
                "id": "{i}",
                "key": "TEST-{i}"
            }}"#
            ))
            .create_async()
            .await;
    }

    // Mock multiple delete calls
    for i in 1..=10 {
        server
            .mock(
                "DELETE",
                format!("/rest/api/{DEFAULT_API_VERSION}/issue/TEST-{i}").as_str(),
            )
            .with_status(204)
            .create_async()
            .await;
    }

    let jira_client = Jira::new(
        url,
        Credentials::Basic("user@example.com".to_string(), "token".to_string()),
    )?;

    // Create 10 issues
    let mut issue_keys = Vec::new();
    for i in 0..10 {
        let issue = jira_client
            .create_issue(
                &jira::models::project::JiraProjectKey { key: "TEST" },
                &format!("Test issue {i}"),
                None,
                vec![],
            )
            .await?;
        issue_keys.push(issue.key);
    }

    assert_eq!(issue_keys.len(), 10);

    // Delete all issues
    for key in issue_keys {
        jira_client.delete_issue(&key).await?;
    }

    Ok(())
}
