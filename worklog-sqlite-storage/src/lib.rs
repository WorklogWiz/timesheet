//! `SQLite` Storage Implementation for Worklog
//!
//! This crate provides a SQLite-based implementation using the
//! `WorklogStorage` trait from `worklog-core`.
//!
//! # Example
//!
//! ```rust,ignore
//! use worklog_core::WorklogStorage;
//! use worklog_sqlite_storage::SqliteStorage;
//!
//! let storage = SqliteStorage::from_url("sqlite:///path/to/db.db", None)?;
//! let worklog_repo = storage.create_worklog_repository();
//! ```

mod migrations;
mod repositories;
mod storage;

pub use migrations::{create_backup, initialize_or_migrate};
pub use storage::SqliteStorage;

// Re-export common types for convenience
pub use worklog_core::WorklogStorage;
