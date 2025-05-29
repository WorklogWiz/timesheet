//!
//! ```rust,ignore
//! // 1. Using the builder with explicit configuration
//! let jira = Jira::builder()
//!     .host("https://your-jira.atlassian.net")
//!     .basic_auth("username@example.com", "your_api_token")
//!     .timeout(30)
//!     .build()
//!     .expect("Failed to create Jira client");
//!
//! // 2. Using environment variables
//! let jira = Jira::builder()
//!     .from_env()
//!     .build()
//!     .expect("Failed to create Jira client");
//!
//! // 3. Direct shortcut for environment variables
//! let jira = JiraBuilder::create_from_env()
//!     .expect("Failed to create Jira client");
//!
//! // 4. Original method (backward compatible)
//! let jira = Jira::new(
//!     "https://your-jira.atlassian.net",
//!     Credentials::Basic("username@example.com".to_string(), "your_api_token".to_string()),
//! ).expect("Failed to create Jira client");
//
//
use crate::{Credentials, Jira};
use log::debug;
use reqwest::Client;
use std::env;
use std::time::Duration;
use thiserror::Error;
use url::Url;

/// Error type for `JiraBuilder` operations
#[derive(Error, Debug)]
pub enum JiraBuilderError {
    #[error("Environment variable {0} not set")]
    EnvVarNotSet(String),

    #[error("URL parsing error: {0}")]
    UrlParseError(#[from] url::ParseError),

    #[error("Invalid URL: {0}")]
    InvalidUrl(String),

    #[error("Jira client initialization error: {0}")]
    ClientInitError(String),

    #[error("Timeout must be positive")]
    InvalidTimeout,
}

/// Names of commonly used environment variables for Jira configuration
pub struct JiraEnvVars;

impl JiraEnvVars {
    pub const HOST: &'static str = "JIRA_HOST";
    pub const USER: &'static str = "JIRA_USER";
    pub const TOKEN: &'static str = "JIRA_TOKEN";
    pub const API_VERSION: &'static str = "JIRA_API_VERSION";
}

pub const DEFAULT_API_VERSION: &str = "latest";

/// Builder for creating Jira client instances with flexible configuration options
pub struct JiraBuilder {
    host: Option<String>,
    api_version: Option<String>,
    credentials: Option<Credentials>,
    timeout: Option<Duration>,
}

impl Default for JiraBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl JiraBuilder {
    /// Creates a new `JiraBuilder` with default configuration
    #[must_use]
    pub fn new() -> Self {
        Self {
            host: None,
            api_version: None,
            credentials: None,
            timeout: None,
        }
    }

    /// Sets the Jira host URL
    #[must_use]
    pub fn host(mut self, host: impl Into<String>) -> Self {
        self.host = Some(host.into());
        self
    }

    #[must_use]
    pub fn credentials(mut self, credentials: Credentials) -> Self {
        self.credentials = Some(credentials);
        self
    }

    /// Sets the API version (default is "3")
    #[must_use]
    pub fn api_version(mut self, version: impl Into<String>) -> Self {
        self.api_version = Some(version.into());
        self
    }

    /// Sets basic authentication credentials
    #[must_use]
    pub fn basic_auth(mut self, username: impl Into<String>, token: impl Into<String>) -> Self {
        self.credentials = Some(Credentials::Basic(username.into(), token.into()));
        self
    }

    /// Sets OAuth/bearer token authentication
    #[must_use]
    pub fn bearer_auth(mut self, token: impl Into<String>) -> Self {
        self.credentials = Some(Credentials::Bearer(token.into()));
        self
    }

    /// Sets a request timeout
    #[must_use]
    pub fn timeout_seconds(self, seconds: u64) -> Self {
        self.timeout(seconds)
    }

    #[must_use]
    pub fn timeout(mut self, seconds: u64) -> Self {
        self.timeout = Some(Duration::from_secs(seconds));
        self
    }

    /// Attempts to load configuration from environment variables
    #[must_use]
    pub fn from_env(self) -> Self {
        let host = env::var(JiraEnvVars::HOST).ok();
        let user = env::var(JiraEnvVars::USER).ok();
        let token = env::var(JiraEnvVars::TOKEN).ok();
        let api_version = env::var(JiraEnvVars::API_VERSION).ok();

        let mut builder = self;

        if let Some(host) = host {
            builder = builder.host(host);
        }

        if let Some(api_version) = api_version {
            builder = builder.api_version(api_version);
        }

        if let (Some(user), Some(token)) = (user, token) {
            builder = builder.basic_auth(user, token);
        }

        builder
    }

    /// Builds a Jira client instance with the configured parameters
    ///
    /// # Errors
    /// Returns `JiraBuilderError` if:
    /// - Required configuration parameters (host, credentials) are not set
    /// - The provided host URL cannot be parsed
    /// - The HTTP client initialization fails
    /// - The timeout value is invalid
    pub fn build(self) -> Result<Jira, JiraBuilderError> {
        // Validate and extract required parameters
        let host = self
            .host
            .ok_or_else(|| JiraBuilderError::EnvVarNotSet(JiraEnvVars::HOST.to_string()))?;

        let credentials = self.credentials.ok_or_else(|| {
            JiraBuilderError::EnvVarNotSet(format!(
                "{} and {}",
                JiraEnvVars::USER,
                JiraEnvVars::TOKEN
            ))
        })?;

        let api_version = self
            .api_version
            .unwrap_or_else(|| DEFAULT_API_VERSION.to_string());

        // Create URL
        let host_url = Url::parse(&host).map_err(JiraBuilderError::UrlParseError)?;

        // Create the HTTP client with a proper configuration
        let mut client_builder = Client::builder();

        if let Some(timeout) = self.timeout {
            client_builder = client_builder.timeout(timeout);
        }

        let client = client_builder
            .build()
            .map_err(|e| JiraBuilderError::ClientInitError(e.to_string()))?;

        // Create the Jira client
        let jira = Jira {
            host: host_url,
            api: format!("rest/api/{api_version}"),
            credentials,
            client,
        };
        debug!("Created Jira client: {jira:#?}");

        Ok(jira)
    }

    /// Creates a new Jira client instance using configuration from environment variables.
    ///
    /// This is a convenience method that combines calling `new()`, `from_env()`, and `build()`
    /// in a single method. It will attempt to read the following environment variables:
    /// - `JIRA_HOST`: The Jira host URL (required)
    /// - `JIRA_USER`: Username for basic authentication
    /// - `JIRA_TOKEN`: API token for basic authentication
    /// - `JIRA_API_VERSION`: API version to use (optional, defaults to "latest")
    ///
    /// # Returns
    /// - `Ok(Jira)` - A configured Jira client instance
    /// - `Err(JiraBuilderError)` - If required environment variables are missing or invalid
    ///
    /// # Errors
    /// Returns `JiraBuilderError` if:
    /// - Required environment variables (`JIRA_HOST`, `JIRA_USER`, `JIRA_TOKEN`) are not set
    /// - The host URL is invalid or cannot be parsed
    /// - Client initialization fails
    ///
    /// # Example
    /// ```no_run
    /// use jira::{Jira, JiraBuilder};
    ///
    /// let jira = JiraBuilder::create_from_env()
    ///     .expect("Failed to create Jira client");
    /// ```
    pub fn create_from_env() -> Result<Jira, JiraBuilderError> {
        Self::new().from_env().build()
    }
}
