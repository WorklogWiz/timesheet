
#[tokio::main]
async fn main() {
    env_logger::init();

    let jira_client = jira_lib::create_jira_client();
    let _results = jira_client.get_time_tracking_options().await;

}
