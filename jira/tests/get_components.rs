mod test_helpers;

use test_helpers::jira_client::create_jira_client;
#[tokio::test]
async fn test_get_components() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let jira_client = create_jira_client();
    let components = jira_client.await.get_components("TWIZ").await?;
    assert!(components.len() > 0, "No components found in project TWIZ");

    Ok(())
}
