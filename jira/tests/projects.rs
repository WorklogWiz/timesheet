use jira::builder::DEFAULT_API_VERSION;
use jira::{Credentials, Jira};
use mockito::Server;

#[tokio::test]
async fn test_get_projects() -> Result<(), Box<dyn std::error::Error>> {
    let mut server = Server::new_async().await;
    let url = server.url();

    // Mock GET /project/search endpoint
    let _m = server
        .mock(
            "GET",
            mockito::Matcher::Regex(format!(
                r"^/rest/api/{DEFAULT_API_VERSION}/project/search\?.*"
            )),
        )
        .with_status(200)
        .with_body(
            r#"{
            "startAt": 0,
            "maxResults": 50,
            "total": 2,
            "isLast": true,
            "values": [
                {
                    "id": "10001",
                    "key": "PROJ1",
                    "name": "Project One",
                    "self": "https://example.atlassian.net/rest/api/2/project/10001",
                    "isPrivate": false
                },
                {
                    "id": "10002",
                    "key": "PROJ2",
                    "name": "Project Two",
                    "self": "https://example.atlassian.net/rest/api/2/project/10002",
                    "isPrivate": false
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

    let projects = jira_client
        .get_projects(vec!["PROJ1".to_string(), "PROJ2".to_string()])
        .await?;

    assert_eq!(projects.len(), 2);
    assert_eq!(projects[0].key, "PROJ1");
    assert_eq!(projects[0].name, "Project One");
    assert_eq!(projects[1].key, "PROJ2");
    assert_eq!(projects[1].name, "Project Two");

    Ok(())
}

#[tokio::test]
async fn test_get_projects_filters_private() -> Result<(), Box<dyn std::error::Error>> {
    let mut server = Server::new_async().await;
    let url = server.url();

    // Mock with one private project
    let _m = server
        .mock(
            "GET",
            mockito::Matcher::Regex(format!(
                r"^/rest/api/{DEFAULT_API_VERSION}/project/search\?.*"
            )),
        )
        .with_status(200)
        .with_body(
            r#"{
            "startAt": 0,
            "maxResults": 50,
            "total": 2,
            "isLast": true,
            "values": [
                {
                    "id": "10001",
                    "key": "PUBLIC",
                    "name": "Public Project",
                    "self": "https://example.atlassian.net/rest/api/2/project/10001",
                    "isPrivate": false
                },
                {
                    "id": "10002",
                    "key": "PRIVATE",
                    "name": "Private Project",
                    "self": "https://example.atlassian.net/rest/api/2/project/10002",
                    "isPrivate": true
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

    let projects = jira_client
        .get_projects(vec!["PUBLIC".to_string(), "PRIVATE".to_string()])
        .await?;

    // Should only return the non-private project
    assert_eq!(projects.len(), 1);
    assert_eq!(projects[0].key, "PUBLIC");

    Ok(())
}
