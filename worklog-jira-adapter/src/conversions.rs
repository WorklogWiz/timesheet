//! Type conversions between Jira and worklog-core types

use worklog_core::{Issue, User, WorklogEntry};

/// Convert a Jira `IssueKey` to a String (for worklog-core)
#[allow(dead_code)]
pub fn issue_key_to_string(jira_key: &jira::models::core::IssueKey) -> String {
    jira_key.to_string()
}

/// Convert a String to a Jira `IssueKey`
#[allow(dead_code)]
pub fn issue_key_from_string(key: &str) -> jira::models::core::IssueKey {
    jira::models::core::IssueKey::new(key)
}

/// Convert a Jira issue to a generic Issue
///
/// Maps Jira-specific fields to the generic domain model:
/// - Jira components → generic tags
/// - Jira issue key → generic key
/// - Summary and description preserved
pub fn issue_from_jira(jira_issue: &jira::models::issue::IssueSummary) -> Issue {
    // Convert Jira components to generic tags
    let tags: Vec<String> = jira_issue
        .fields
        .components
        .iter()
        .map(|component| component.name.clone())
        .collect();

    Issue {
        key: jira_issue.key.to_string(),
        summary: jira_issue.fields.summary.clone(),
        description: None, // IssueSummary doesn't include description
        tags,
        provider_id: Some(jira_issue.id.clone()),
    }
}

/// Convert a Jira user to a generic User
pub fn user_from_jira(jira_user: jira::models::user::User) -> User {
    User {
        id: jira_user.account_id,
        display_name: jira_user.display_name,
        email: Some(jira_user.email_address),
        timezone: Some(jira_user.time_zone),
    }
}

/// Convert Jira Worklog to `WorklogEntry`
#[allow(dead_code)]
pub fn worklog_entry_from_jira(
    jira_worklog: &jira::models::worklog::Worklog,
    issue_key: &str,
) -> WorklogEntry {
    use chrono::Local;

    let now = Local::now();
    WorklogEntry {
        id: None,
        issue_key: Some(issue_key.to_string()),
        started_at: jira_worklog.started.with_timezone(&Local),
        stopped_at: Some(
            (jira_worklog.started
                + chrono::Duration::seconds(i64::from(jira_worklog.timeSpentSeconds)))
            .with_timezone(&Local),
        ),
        comment: jira_worklog.comment.clone(),
        tags: Vec::new(),
        time_spent_seconds: Some(jira_worklog.timeSpentSeconds),
        synced_to_provider: true,
        provider_worklog_id: Some(jira_worklog.id.clone()),
        created_at: jira_worklog.created.with_timezone(&Local),
        updated_at: jira_worklog.updated.with_timezone(&Local),
        deleted_at: None,
        last_synced_at: Some(now), // Just synced from Jira
        version: 1,
        has_conflict: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jira::models::{
        core::Fields, core::IssueKey, issue::IssueSummary, project::Component,
        user::User as JiraUser,
    };

    #[test]
    fn test_issue_from_jira() {
        let jira_issue = IssueSummary {
            id: "12345".to_string(),
            key: IssueKey::new("PROJ-123"),
            fields: Fields {
                summary: "Test issue".to_string(),
                components: vec![
                    Component {
                        id: "1".to_string(),
                        name: "Backend".to_string(),
                    },
                    Component {
                        id: "2".to_string(),
                        name: "API".to_string(),
                    },
                ],
            },
        };

        let issue = issue_from_jira(&jira_issue);

        assert_eq!(issue.key, "PROJ-123");
        assert_eq!(issue.summary, "Test issue");
        assert_eq!(issue.tags, vec!["Backend", "API"]);
        assert_eq!(issue.provider_id, Some("12345".to_string()));
    }

    #[test]
    fn test_issue_from_jira_no_components() {
        let jira_issue = IssueSummary {
            id: "12345".to_string(),
            key: IssueKey::new("PROJ-456"),
            fields: Fields {
                summary: "No components".to_string(),
                components: vec![],
            },
        };

        let issue = issue_from_jira(&jira_issue);

        assert_eq!(issue.key, "PROJ-456");
        assert!(issue.tags.is_empty());
    }

    #[test]
    fn test_user_from_jira() {
        let jira_user = JiraUser {
            self_url: "http://example.com".to_string(),
            account_id: "user123".to_string(),
            display_name: "John Doe".to_string(),
            email_address: "john@example.com".to_string(),
            time_zone: "America/New_York".to_string(),
        };

        let user = user_from_jira(jira_user);

        assert_eq!(user.id, "user123");
        assert_eq!(user.display_name, "John Doe");
        assert_eq!(user.email, Some("john@example.com".to_string()));
        assert_eq!(user.timezone, Some("America/New_York".to_string()));
    }
}
