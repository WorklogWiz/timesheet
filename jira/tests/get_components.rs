use jira::builder::DEFAULT_API_VERSION;
use jira::{Credentials, Jira};
use mockito::Server;

#[tokio::test]
async fn test_get_components() -> Result<(), Box<dyn std::error::Error>> {
    let mut server = Server::new_async().await;
    let url = server.url();

    let _m = server
        .mock(
            "GET",
            format!("/rest/api/{DEFAULT_API_VERSION}/project/TWIZ/components?componentSource=auto")
                .as_str(),
        )
        .with_status(200)
        .with_body(
            r#"[
            {
                "id": "10001",
                "name": "Component A"
            },
            {
                "id": "10002",
                "name": "Component B"
            },
            {
                "id": "10003",
                "name": "Component C"
            }
        ]"#,
        )
        .create_async()
        .await;

    let jira_client = Jira::new(
        url,
        Credentials::Basic("user@example.com".to_string(), "token".to_string()),
    )?;

    let components = jira_client.get_components("TWIZ").await?;
    assert!(
        !components.is_empty(),
        "No components found in project TWIZ"
    );
    assert_eq!(components.len(), 3);
    assert_eq!(components[0].name, "Component A");
    assert_eq!(components[1].name, "Component B");
    assert_eq!(components[2].name, "Component C");

    Ok(())
}
