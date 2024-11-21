use chrono::DateTime;
use jira_lib::{Author, JiraFields, JiraIssue, JiraKey, JiraProject, Worklog};

#[test]
fn test_collect_authors() {
    let p = JiraProject {
        id: String::new(),
        key: String::new(),
        name: String::new(),
        url: String::new(),
        is_private: false,
        issues: vec![JiraIssue {
            id: String::new(),
            self_url: String::new(),
            key: JiraKey::from("key-1"),
            fields: JiraFields {
                summary: "Rubbish".to_string(),
                asset: Option::None,
            },
            worklogs: vec![
                Worklog {
                    author: Author {
                        accountId: "1".to_string(),
                        emailAddress: None,
                        displayName: "Steinar".to_string(),
                    },
                    id: String::new(),
                    created: DateTime::default(),
                    updated: DateTime::default(),
                    started: DateTime::default(),
                    timeSpent: String::new(),
                    timeSpentSeconds: 0,
                    issueId: String::new(),
                    comment: None,
                },
                Worklog {
                    author: Author {
                        accountId: "1".to_string(),
                        emailAddress: None,
                        displayName: "Steinar".to_string(),
                    },
                    id: String::new(),
                    created: DateTime::default(),
                    updated: DateTime::default(),
                    started: DateTime::default(),
                    timeSpent: String::new(),
                    timeSpentSeconds: 0,
                    issueId: String::new(),
                    comment: None,
                },
                Worklog {
                    author: Author {
                        accountId: "2".to_string(),
                        emailAddress: None,
                        displayName: "Johanne".to_string(),
                    },
                    id: String::new(),
                    created: DateTime::default(),
                    updated: DateTime::default(),
                    started: DateTime::default(),
                    timeSpent: String::new(),
                    timeSpentSeconds: 0,
                    issueId: String::new(),
                    comment: None,
                },
            ],
        }],
    };

    let authors: Vec<Author> = p
        .issues
        .iter()
        .flat_map(|i| &i.worklogs)
        .map(|w| &w.author)
        .cloned()
        .collect();
    assert_eq!(authors[0].displayName, "Steinar");
    assert_eq!(authors[1].displayName, "Steinar");
    assert_eq!(authors.len(), 3);
}
