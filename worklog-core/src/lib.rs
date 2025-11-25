//! # worklog-core
//!
//! Pure domain logic for worklog time tracking.
//!
//! This crate contains:
//! - **Domain types**: `WorklogEntry`, `Issue`, `User`, `IssueKey`
//! - **Traits**: `IssueTrackerClient`, repository traits
//! - **Business logic**: Validation, time calculations
//! - **Errors**: Domain error types
//!
//! ## Philosophy
//!
//! This crate has **zero infrastructure dependencies**. It defines what a worklog
//! system *is* and *does*, without caring about *how* it's implemented.
//!
//! - No Jira dependency (Jira-specific code lives in `worklog-jira-adapter`)
//! - No database dependency (storage implementations live in storage crates)
//! - No HTTP clients, no external services
//!
//! ## Key Concepts
//!
//! ### Unified `WorklogEntry`
//!
//! There is no separate "Timer" type. A worklog entry with `stopped_at = None`
//! represents an active timer. When stopped, `stopped_at` is set.
//!
//! ```rust
//! use worklog_core::domain::WorklogEntry;
//! use chrono::Local;
//!
//! let mut entry = WorklogEntry::start_now(
//!     Some("PROJ-123".to_string()),
//!     Some("Working on feature".to_string())
//! );
//!
//! assert!(entry.is_active());  // It's a "timer"
//!
//! entry.stop(Local::now());
//! assert!(!entry.is_active()); // Now it's a "completed worklog"
//! ```
//!
//! ### Optional Issue Assignment
//!
//! Worklog entries don't require an issue assignment. You can track work locally
//! without linking to an external issue tracker:
//!
//! ```rust
//! use worklog_core::domain::WorklogEntry;
//!
//! // Local-only tracking (no issue)
//! let entry = WorklogEntry::start_now(
//!     None,  // No issue key
//!     Some("Learning Rust".to_string())
//! );
//! ```
//!
//! ### Provider-Agnostic
//!
//! All types work with any issue tracking system (Jira, GitHub, Linear, etc.).
//! Provider-specific code belongs in adapter crates.

pub mod domain;
pub mod error;
pub mod services;
pub mod traits;

// Re-export commonly used types
pub use domain::{
    Issue, IssueKey, RemoteWorklog, SyncConflict, SyncResult, SyncState, User, WorklogEntry,
};
pub use error::{WorklogError, WorklogResult};
pub use services::{IssueService, SyncService, UserService, WorklogService};
pub use traits::{
    IssueRepository, IssueTrackerClient, UserRepository, WorklogRepository, WorklogStorage,
};
