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
use reqwest::Client;
use std::env;
use std::time::Duration;
use log::debug;
use thiserror::Error;
use url::Url;

/// Error type for JiraBuilder operations
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

pub const DEFAULT_API_VERSION: &'static str = "latest";

/// Builder for creating Jira client instances with flexible configuration options
pub struct JiraBuilder {
    host: Option<String>,
    api_version: Option<String>,
    credentials: Option<Credentials>,
    timeout: Option<Duration>,
    client_config: Option<Box<dyn Fn(&mut reqwest::ClientBuilder) -> reqwest::ClientBuilder>>,
}

impl Default for JiraBuilder {
    fn default() -> Self {
        Self::new()
    }
}


impl JiraBuilder {
    /// Creates a new JiraBuilder with default configuration
    pub fn new() -> Self {
        Self {
            host: None,
            api_version: None,
            credentials: None,
            timeout: None,
            client_config: None,
        }
    }

    /// Sets the Jira host URL
    pub fn host(mut self, host: impl Into<String>) -> Self {
        self.host = Some(host.into());
        self
    }
    
    pub fn credentials(mut self, credentials: Credentials) -> Self {
        self.credentials = Some(credentials);
        self
    }

    /// Sets the API version (default is "3")
    pub fn api_version(mut self, version: impl Into<String>) -> Self {
        self.api_version = Some(version.into());
        self
    }

    /// Sets basic authentication credentials
    pub fn basic_auth(mut self, username: impl Into<String>, token: impl Into<String>) -> Self {
        self.credentials = Some(Credentials::Basic(username.into(), token.into()));
        self
    }

    /// Sets OAuth/bearer token authentication
    pub fn bearer_auth(mut self, token: impl Into<String>) -> Self {
        self.credentials = Some(Credentials::Bearer(token.into()));
        self
    }

    /// Sets a request timeout
    pub fn timeout(mut self, seconds: u64) -> Self {
        self.timeout = Some(Duration::from_secs(seconds));
        self
    }

    /// Advanced configuration of the underlying reqwest client
    pub fn configure_client<F>(mut self, config_fn: F) -> Self
    where
        F: Fn(&mut reqwest::ClientBuilder) -> reqwest::ClientBuilder + 'static,
    {
        self.client_config = Some(Box::new(config_fn));
        self
    }

    /// Attempts to load configuration from environment variables
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
    pub fn build(self) -> Result<Jira, JiraBuilderError> {
        // Validate and extract required parameters
        let host = self.host.ok_or_else(|| JiraBuilderError::EnvVarNotSet(JiraEnvVars::HOST.to_string()))?;

        let credentials = self.credentials.ok_or_else(|| {
            JiraBuilderError::EnvVarNotSet(format!("{} and {}", JiraEnvVars::USER, JiraEnvVars::TOKEN))
        })?;

        let api_version = self.api_version.unwrap_or_else(|| DEFAULT_API_VERSION.to_string());

        // Create URL
        let host_url = Url::parse(&host)
            .map_err(JiraBuilderError::UrlParseError)?;

        // Create the HTTP client with a proper configuration
        let mut client_builder = Client::builder();

        if let Some(timeout) = self.timeout {
            client_builder = client_builder.timeout(timeout);
        }

        if let Some(config_fn) = self.client_config {
            client_builder = config_fn(&mut client_builder);
        }

        let client = client_builder.build()
            .map_err(|e| JiraBuilderError::ClientInitError(e.to_string()))?;

        // Create the Jira client
        let jira = Jira {
            host: host_url,
            api: format!("rest/api/{api_version}"),
            credentials,
            client,
        };
        debug!("Created Jira client: {:#?}", jira);
        
        Ok(jira)
    }

    /// Convenience method to create a Jira client from environment variables
    pub fn create_from_env() -> Result<Jira, JiraBuilderError> {
        Self::new().from_env().build()
    }
}

// Example extension methods for Jira to make the transition easier
impl Jira {
    /// Create a Jira client builder
    pub fn builder() -> JiraBuilder {
        JiraBuilder::new()
    }
}