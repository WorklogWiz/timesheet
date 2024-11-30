use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    #[serde(alias = "self")]
    pub self_url: String,
    #[serde(alias = "accountId")]
    pub account_id: String,
    #[serde(alias = "emailAddress")]
    pub email_address: String,
    #[serde(alias = "displayName")]
    pub display_name: String,
    #[serde(alias = "timeZone")]
    pub time_zone: String,
}
