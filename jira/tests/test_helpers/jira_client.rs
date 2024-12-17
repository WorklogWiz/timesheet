use jira::{Credentials, Jira};

#[cfg(test)]
pub async fn create_jira_client() -> Jira {
    let (host, user, token) = get_jira_params();
    let client =
        Jira::new(&host, Credentials::Basic(user, token)).expect("Error initializing jira client");
    client
}
///
/// Manages the secrets needed to access the Jira instance used for integration testing.
///
/// This is how you include it in your code
/// ```
/// #[cfg(test)]
/// use test_helpers::credentials;
/// ```

/// Names the environment variables used to obtain the credentials needed for running
/// integration tests
pub const JIRA_HOST: &str = "JIRA_HOST";
pub const JIRA_USER: &str = "JIRA_USER";
pub const JIRA_TOKEN: &str = "JIRA_TOKEN";

pub(crate) fn get_jira_host() -> String {
    std::env::var(JIRA_HOST).expect(&format!("Environment variable {JIRA_HOST} not set. Set it to something like 'https://norn.jira.atlassian.com'"))
}

pub(crate) fn get_jira_user() -> String {
    std::env::var(JIRA_USER).expect(&format!(
        "Environment variable {JIRA_USER} not set. Set it to something like 'user@domain.com'"
    ))
}

pub(crate) fn get_jira_token() -> String {
    std::env::var(JIRA_TOKEN).expect(&format!(
        "Environment variable {JIRA_TOKEN} not set. Set it to something like 'secret'"
    ))
}

#[cfg(test)]
/// Convenience function to obtain all the jira parameters required, in one go
pub(crate) fn get_jira_params() -> (String, String, String) {
    (get_jira_host(), get_jira_user(), get_jira_token())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_credentials_from_env() {
        let jira_host = get_jira_host();
        let jira_user = get_jira_user();
        let jira_token = get_jira_token();
        assert!(!jira_host.is_empty());
        assert!(!jira_user.is_empty());
        assert!(!jira_token.is_empty());
        println!("Jira host: {}", jira_host);
        println!("Jira user: {}", jira_user);
        println!("Jira token: {}", jira_token);
    }
}
