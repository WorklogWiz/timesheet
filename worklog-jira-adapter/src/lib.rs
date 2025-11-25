//! # worklog-jira-adapter
//!
//! Jira adapter for the worklog time tracking system.
//!
//! This crate implements `worklog_core::IssueTrackerClient` for Atlassian Jira,
//! providing integration between the generic worklog domain and Jira's specific API.
//!
//! ## Features
//!
//! - Implements `IssueTrackerClient` trait for Jira
//! - Converts Jira-specific types to generic domain types
//! - Maps Jira components to generic tags
//! - Handles Jira authentication and API communication
//!
//! ## Example
//!
//! ```rust,no_run
//! use worklog_jira_adapter::JiraAdapter;
//! use worklog_core::traits::IssueTrackerClient;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create Jira adapter
//! let adapter = JiraAdapter::from_config(
//!     "https://your-domain.atlassian.net",
//!     "user@example.com",
//!     "api_token"
//! )?;
//!
//! // Use it as a generic issue tracker
//! let issue = adapter.get_issue("PROJ-123").await?;
//! println!("Issue: {} - {}", issue.key, issue.summary);
//! # Ok(())
//! # }
//! ```

mod adapter;
mod conversions;

pub use adapter::JiraAdapter;
