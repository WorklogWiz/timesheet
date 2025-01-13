use rusqlite::Connection;
use std::sync::{Arc, Mutex};

// Application repository modules, each representing specific database entity operations.
pub(crate) mod component_repository;
pub(crate) mod issue_repository;
pub(crate) mod user_repository;
pub(crate) mod worklog_repository;

// Database-related utilities and managers.
pub(crate) mod database_manager;
pub(crate) mod sqlite;

/// A thread-safe, shared connection to an ``SQLite`` database,
/// used across multiple repository layers.
pub(crate) type SharedSqliteConnection = Arc<Mutex<Connection>>;
