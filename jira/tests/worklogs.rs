use chrono::{Days, Local, NaiveDateTime};
use jira::builder::DEFAULT_API_VERSION;
use jira::models::core::IssueKey;
use jira::{Credentials, Jira};
use mockito::Server;

#[tokio::test]
async fn test_insert_worklog() -> Result<(), Box<dyn std::error::Error>> {
    let mut server = Server::new_async().await;
    let url = server.url();

    // Mock POST /issue/{issueIdOrKey}/worklog
    let _m = server
        .mock(
            "POST",
            format!("/rest/api/{DEFAULT_API_VERSION}/issue/TEST-123/worklog").as_str(),
        )
        .with_status(201)
        .with_body(
            r#"{
            "id": "10001",
            "author": {
                "accountId": "user123",
                "displayName": "Test User",
                "emailAddress": "user@example.com"
            },
            "created": "2024-01-15T10:00:00.000+0000",
            "updated": "2024-01-15T10:00:00.000+0000",
            "started": "2024-01-15T09:00:00.000+0000",
            "timeSpent": "2h",
            "timeSpentSeconds": 7200,
            "issueId": "12345",
            "comment": "Working on feature X"
        }"#,
        )
        .create_async()
        .await;

    let jira_client = Jira::new(
        url,
        Credentials::Basic("user@example.com".to_string(), "token".to_string()),
    )?;

    let started = Local::now().checked_sub_days(Days::new(1)).unwrap();
    let worklog = jira_client
        .insert_worklog("TEST-123", started, 7200, "Working on feature X")
        .await?;

    assert_eq!(worklog.id, "10001");
    assert_eq!(worklog.timeSpentSeconds, 7200);
    assert_eq!(worklog.comment, Some("Working on feature X".to_string()));

    Ok(())
}

#[tokio::test]
async fn test_delete_worklog() -> Result<(), Box<dyn std::error::Error>> {
    let mut server = Server::new_async().await;
    let url = server.url();

    // Mock DELETE /issue/{issueIdOrKey}/worklog/{id}
    let _m = server
        .mock(
            "DELETE",
            format!("/rest/api/{DEFAULT_API_VERSION}/issue/TEST-123/worklog/10001").as_str(),
        )
        .with_status(204)
        .create_async()
        .await;

    let jira_client = Jira::new(
        url,
        Credentials::Basic("user@example.com".to_string(), "token".to_string()),
    )?;

    let result = jira_client
        .delete_worklog("TEST-123".to_string(), "10001".to_string())
        .await;

    assert!(result.is_ok());

    Ok(())
}

#[tokio::test]
async fn test_get_work_logs_for_issue() -> Result<(), Box<dyn std::error::Error>> {
    let mut server = Server::new_async().await;
    let url = server.url();

    // Mock GET /issue/{issueIdOrKey}/worklog
    let _m = server
        .mock(
            "GET",
            mockito::Matcher::Regex(format!(
                r"^/rest/api/{DEFAULT_API_VERSION}/issue/TEST-123/worklog\?.*"
            )),
        )
        .with_status(200)
        .with_body(
            r#"{
            "startAt": 0,
            "maxResults": 5000,
            "total": 2,
            "worklogs": [
                {
                    "id": "10001",
                    "author": {
                        "accountId": "user123",
                        "displayName": "Test User",
                        "emailAddress": "user@example.com"
                    },
                    "created": "2024-01-15T10:00:00.000+0000",
                    "updated": "2024-01-15T10:00:00.000+0000",
                    "started": "2024-01-15T09:00:00.000+0000",
                    "timeSpent": "2h",
                    "timeSpentSeconds": 7200,
                    "issueId": "12345",
                    "comment": "Working on feature X"
                },
                {
                    "id": "10002",
                    "author": {
                        "accountId": "user456",
                        "displayName": "Another User",
                        "emailAddress": "another@example.com"
                    },
                    "created": "2024-01-16T10:00:00.000+0000",
                    "updated": "2024-01-16T10:00:00.000+0000",
                    "started": "2024-01-16T09:00:00.000+0000",
                    "timeSpent": "1h",
                    "timeSpentSeconds": 3600,
                    "issueId": "12345",
                    "comment": "Testing feature X"
                }
            ]
        }"#,
        )
        .create_async()
        .await;

    let jira_client = Jira::new(
        url,
        Credentials::Basic("user@example.com".to_string(), "token".to_string()),
    )?;

    let started_after = NaiveDateTime::parse_from_str("2024-01-01 00:00:00", "%Y-%m-%d %H:%M:%S")?;
    let worklogs = jira_client
        .get_work_logs_for_issue(&IssueKey::new("TEST-123"), started_after)
        .await?;

    assert_eq!(worklogs.len(), 2);
    assert_eq!(worklogs[0].id, "10001");
    assert_eq!(worklogs[0].timeSpentSeconds, 7200);
    assert_eq!(worklogs[1].id, "10002");
    assert_eq!(worklogs[1].timeSpentSeconds, 3600);

    Ok(())
}

#[tokio::test]
async fn test_get_work_logs_for_current_user() -> Result<(), Box<dyn std::error::Error>> {
    let mut server = Server::new_async().await;
    let url = server.url();

    // Mock GET /myself
    let _m_user = server
        .mock(
            "GET",
            format!("/rest/api/{DEFAULT_API_VERSION}/myself").as_str(),
        )
        .with_status(200)
        .with_body(
            r#"{
            "self": "https://example.atlassian.net/rest/api/2/user?accountId=user123",
            "accountId": "user123",
            "emailAddress": "user@example.com",
            "displayName": "Test User",
            "timeZone": "America/New_York"
        }"#,
        )
        .create_async()
        .await;

    // Mock GET /issue/{issueIdOrKey}/worklog with multiple users
    let _m_worklogs = server
        .mock(
            "GET",
            mockito::Matcher::Regex(format!(
                r"^/rest/api/{DEFAULT_API_VERSION}/issue/TEST-123/worklog\?.*"
            )),
        )
        .with_status(200)
        .with_body(
            r#"{
            "startAt": 0,
            "maxResults": 5000,
            "total": 3,
            "worklogs": [
                {
                    "id": "10001",
                    "author": {
                        "accountId": "user123",
                        "displayName": "Test User",
                        "emailAddress": "user@example.com"
                    },
                    "created": "2024-01-15T10:00:00.000+0000",
                    "updated": "2024-01-15T10:00:00.000+0000",
                    "started": "2024-01-15T09:00:00.000+0000",
                    "timeSpent": "2h",
                    "timeSpentSeconds": 7200,
                    "issueId": "12345",
                    "comment": "My work"
                },
                {
                    "id": "10002",
                    "author": {
                        "accountId": "user456",
                        "displayName": "Another User",
                        "emailAddress": "another@example.com"
                    },
                    "created": "2024-01-16T10:00:00.000+0000",
                    "updated": "2024-01-16T10:00:00.000+0000",
                    "started": "2024-01-16T09:00:00.000+0000",
                    "timeSpent": "1h",
                    "timeSpentSeconds": 3600,
                    "issueId": "12345",
                    "comment": "Someone else's work"
                },
                {
                    "id": "10003",
                    "author": {
                        "accountId": "user123",
                        "displayName": "Test User",
                        "emailAddress": "user@example.com"
                    },
                    "created": "2024-01-17T10:00:00.000+0000",
                    "updated": "2024-01-17T10:00:00.000+0000",
                    "started": "2024-01-17T09:00:00.000+0000",
                    "timeSpent": "3h",
                    "timeSpentSeconds": 10800,
                    "issueId": "12345",
                    "comment": "More of my work"
                }
            ]
        }"#,
        )
        .create_async()
        .await;

    let jira_client = Jira::new(
        url,
        Credentials::Basic("user@example.com".to_string(), "token".to_string()),
    )?;

    let worklogs = jira_client
        .get_work_logs_for_current_user("TEST-123", None)
        .await?;

    // Should only return worklogs for user123 (filtered out user456)
    assert_eq!(worklogs.len(), 2);
    assert_eq!(worklogs[0].id, "10001");
    assert_eq!(worklogs[0].author.accountId, "user123");
    assert_eq!(worklogs[1].id, "10003");
    assert_eq!(worklogs[1].author.accountId, "user123");

    Ok(())
}
