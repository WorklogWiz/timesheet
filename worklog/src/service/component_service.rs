use crate::error::WorklogError;
use crate::repository::component_repository::ComponentRepository;
use jira::models::core::IssueKey;
use jira::models::project::Component;
use std::sync::Arc;

pub struct ComponentService<R: ComponentRepository> {
    repository: Arc<R>,
}

impl<R: ComponentRepository> ComponentService<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self { repository }
    }

    pub fn create_component(
        &self,
        issue_key: &IssueKey,
        components: &Vec<Component>,
    ) -> Result<(), WorklogError> {
        self.repository.create_component(issue_key, components)
    }
}
