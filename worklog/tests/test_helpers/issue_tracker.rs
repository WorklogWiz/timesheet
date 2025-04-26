//! The issue tracker keeps track of issues created by the tests and deletes them afterwards.

use jira::models::core::IssueKey;
use jira::Jira;

pub struct IssueTracker {
    pub(crate) issue_keys: Vec<IssueKey>,
}

impl IssueTracker {
    pub fn new() -> Self {
        Self {
            issue_keys: Vec::new(),
        }
    }

    pub fn track(&mut self, issue_key: IssueKey) {
        self.issue_keys.push(issue_key);
    }

    pub async fn cleanup(&mut self, jira_client: &Jira) {
        for key in &self.issue_keys {
            if let Err(e) = jira_client.delete_issue(key).await {
                eprintln!("Error deleting issue during cleanup() {}: {}", key, e);
            } else {
                println!("cleanup() :- Deleted jira issue {}", key);
            };
        }
        self.issue_keys.clear();
        assert!(self.is_clean());
    }

    pub fn is_clean(&self) -> bool {
        self.issue_keys.is_empty()
    }

    #[allow(dead_code)]
    pub fn last_key(&self) -> Option<IssueKey> {
        self.issue_keys.last().cloned()
    }
}
