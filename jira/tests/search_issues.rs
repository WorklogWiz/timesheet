use jira::builder::DEFAULT_API_VERSION;
use jira::{Credentials, Jira};
use mockito::Server;

#[tokio::test]
async fn test_search_issues() -> Result<(), Box<dyn std::error::Error>> {
    let mut server = Server::new_async().await;
    let url = server.url();

    // Mock search endpoint
    let _m_search = server
        .mock(
            "GET",
            mockito::Matcher::Regex(format!(r"^/rest/api/{DEFAULT_API_VERSION}/search/jql\?.*")),
        )
        .with_status(200)
        .with_body(
            r#"{
            "issues": [
                {
                    "id": "1",
                    "key": "TEST-1",
                    "fields": {
                        "summary": "Test issue 1",
                        "components": [
                            {"id": "comp-1", "name": "Component 1"}
                        ]
                    }
                },
                {
                    "id": "2",
                    "key": "TEST-2",
                    "fields": {
                        "summary": "Test issue 2",
                        "components": [
                            {"id": "comp-1", "name": "Component 1"}
                        ]
                    }
                },
                {
                    "id": "3",
                    "key": "TEST-3",
                    "fields": {
                        "summary": "Test issue 3",
                        "components": [
                            {"id": "comp-2", "name": "Component 2"}
                        ]
                    }
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

    let search_result = jira_client
        .get_issue_summaries(&["TEST"], &[], Some(true))
        .await?;

    assert!(!search_result.is_empty());
    assert_eq!(search_result.len(), 3);

    // Verify the first issue has components
    if let Some(first_issue) = search_result.first() {
        assert!(
            !first_issue.fields.components.is_empty(),
            "The first issue does not have any components."
        );
    } else {
        panic!("No issues were returned in the search result.");
    }

    Ok(())
}
