//! Traits for worklog domain

mod issue_tracker;
mod repository;
mod storage;

pub use issue_tracker::IssueTrackerClient;
pub use repository::{IssueRepository, UserRepository, WorklogRepository};
pub use storage::WorklogStorage;

#[cfg(test)]
pub use issue_tracker::MockIssueTrackerClient;
#[cfg(test)]
pub use repository::{MockIssueRepository, MockUserRepository, MockWorklogRepository};
