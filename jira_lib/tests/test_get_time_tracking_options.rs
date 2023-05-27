use tokio;

use jira_lib::{http_client};

#[tokio::test]
async fn test_get_time_tracking_options() {
    let http_client = http_client();
    let options = jira_lib::get_time_tracking_options(&http_client).await;
    assert_eq!(options.defaultUnit, "hour".to_string());
}