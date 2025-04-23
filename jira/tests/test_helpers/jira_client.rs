use jira::builder::JiraBuilder;
use jira::Jira;

#[cfg(test)]
pub fn create() -> Jira {
    JiraBuilder::create_from_env().expect("Error initializing jira client")
}
