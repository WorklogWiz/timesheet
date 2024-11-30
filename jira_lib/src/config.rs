use serde::{Deserialize, Serialize};

pub const JIRA_TOKEN_STORED_IN_MACOS_KEYCHAIN: &str = "*** stored in macos keychain ***";

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct JiraClientConfiguration {
    pub jira_url: String,
    pub user: String,
    pub token: String,
}

impl Default for JiraClientConfiguration {
    fn default() -> Self {
        JiraClientConfiguration {
            jira_url: "https://autostore.atlassian.net/rest/api/latest".into(),
            user: "user.name@autostoresystem.com".into(),
            token: "<your secret jira token goes here>".into(),
        }
    }
}

impl JiraClientConfiguration {
    /// Does the token look like a valid Jira Security token?
    #[must_use]
    pub fn has_valid_jira_token(&self) -> bool {
        !(self.token == JiraClientConfiguration::default().token
            || self.token.contains("secret")
            || self.token == JIRA_TOKEN_STORED_IN_MACOS_KEYCHAIN)
    }
}
