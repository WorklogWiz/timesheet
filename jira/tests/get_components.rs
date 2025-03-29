mod test_helpers;

use test_helpers::jira_client;
#[tokio::test]
async fn test_get_components() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let jira_client = jira_client::create();
    let components = jira_client.get_components("TWIZ").await?;
    assert!(
        !components.is_empty(),
        "No components found in project TWIZ"
    );

    Ok(())
}
