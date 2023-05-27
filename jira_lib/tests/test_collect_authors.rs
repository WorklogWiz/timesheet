use jira_lib::{Author, JiraFields, JiraIssue, JiraProject, Worklog};

#[test]
fn test_collect_authors() {
    let p = JiraProject {
        id: "".to_string(),
        key: "".to_string(),
        name: "".to_string(),
        url: "".to_string(),
        is_private: false,
        issues: vec![JiraIssue {
            id: "".to_string(),
            self_url: "".to_string(),
            key: "".to_string(),
            fields: JiraFields {summary: "Rubbish".to_string(),asset: Option::None },
            worklogs: vec![
                Worklog {
                    author: Author {
                        accountId: "1".to_string(),
                        emailAddress: None,
                        displayName: "Steinar".to_string(),
                    },
                    id: "".to_string(),
                    created: Default::default(),
                    updated: Default::default(),
                    started: Default::default(),
                    timeSpent: "".to_string(),
                    timeSpentSeconds: 0,
                    issueId: "".to_string(),
                },
                Worklog {
                    author: Author {
                        accountId: "1".to_string(),
                        emailAddress: None,
                        displayName: "Steinar".to_string(),
                    },
                    id: "".to_string(),
                    created: Default::default(),
                    updated: Default::default(),
                    started: Default::default(),
                    timeSpent: "".to_string(),
                    timeSpentSeconds: 0,
                    issueId: "".to_string(),
                },
                Worklog {
                    author: Author {
                        accountId: "2".to_string(),
                        emailAddress: None,
                        displayName: "Johanne".to_string(),
                    },
                    id: "".to_string(),
                    created: Default::default(),
                    updated: Default::default(),
                    started: Default::default(),
                    timeSpent: "".to_string(),
                    timeSpentSeconds: 0,
                    issueId: "".to_string(),
                },
            ],
        }],
    };

    let authors: Vec<Author> = p.issues.iter().flat_map(|i| &i.worklogs).map(|w| &w.author).cloned().collect();
    assert_eq!(authors[0].displayName, "Steinar");
    assert_eq!(authors[1].displayName, "Steinar");
    assert_eq!(authors.len(),3);
}
