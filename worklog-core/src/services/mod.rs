//! Domain services for the worklog application
//!
//! Services orchestrate business logic using repository traits and domain types.
//! They belong in the core because they implement domain rules and don't depend
//! on any infrastructure (no databases, no HTTP, no files).

pub mod issue;
pub mod sync;
pub mod user;
pub mod worklog;

// Re-export the services
pub use issue::IssueService;
pub use sync::SyncService;
pub use user::UserService;
pub use worklog::WorklogService;
