use jira::{ Jira};

#[cfg(test)]
pub fn create() -> Jira {
    Jira::builder().from_env().build().expect("Error initializing jira client")
}