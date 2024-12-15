use crate::test_helpers::credentials::get_jira_params;
use jira::{Credentials, Jira};

#[allow(dead_code)]
pub async fn create_jira_client() -> Jira {
    let (host, user, token) = get_jira_params();
    let client =
        Jira::new(&host, Credentials::Basic(user, token)).expect("Error initializing jira client");
    client
}
