// worklog/tests/test_helpers/fixtures.rs

use chrono::{Duration, Local, Utc};
use jira::models::core::{Fields, IssueKey};
use jira::models::issue::IssueSummary;
use worklog::types::{LocalWorklog, Timer};

/// Constants for test data
pub const TEST_ISSUE_KEY: &str = "TEST-123";
pub const TEST_ISSUE_SUMMARY: &str = "Test issue for integration tests";

/// Creates a sample Timer for testing
pub fn create_test_timer(issue_key: &str, active: bool) -> Timer {
    let now = Utc::now().with_timezone(&Local);
    let started_time = now - Duration::hours(1);

    Timer {
        id: None,
        issue_key: issue_key.to_string(),
        created_at: started_time,
        started_at: started_time,
        stopped_at: if active {
            None
        } else {
            Some(now.with_timezone(&Local))
        },
        synced: false,
        comment: Some("Test timer comment".to_string()),
    }
}

#[cfg(test)]
#[allow(dead_code)]
pub fn create_worklog_entry(issue_key: IssueKey) -> LocalWorklog {
    LocalWorklog {
        id: "123456789".to_string(),
        issue_key,
        created: Local::now(),
        updated: Local::now(),
        started: Local::now() - Duration::hours(1),
        timeSpent: "3600".to_string(),
        timeSpentSeconds: 3600,
        issueId: 0,
        author: "".to_string(),
        comment: None,
    }
}

/// Creates a sample IssueSummary for testing
pub fn create_test_issue_info() -> IssueSummary {
    IssueSummary {
        id: "123".into(),
        key: IssueKey::from(TEST_ISSUE_KEY.to_string()),
        fields: Fields {
            summary: TEST_ISSUE_SUMMARY.to_string(),
            components: vec![],
        },
    }
}

/// Creates a set of test issues for the database

#[cfg(test)]
#[allow(dead_code)]
pub fn create_test_issues() -> Vec<IssueSummary> {
    vec![
        create_test_issue_info(),
        IssueSummary {
            id: "124".into(),
            key: IssueKey::from("TEST-124"),
            fields: Fields {
                summary: "Another test issue".to_string(),
                components: vec![],
            },
        },
        IssueSummary {
            id: "125".into(),
            key: IssueKey::from("TEST-125"),
            fields: Fields {
                summary: "Yet another test issue".to_string(),
                components: vec![],
            },
        },
    ]
}
