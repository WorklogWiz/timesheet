//! Service for managing Jira components within a specific issue.
//!
//! The `ComponentService` provides capabilities to create components related to an issue
//! by utilizing a repository implementation.
//!
//! # Examples
//!
//! ```rust,ignore
//! use crate::repository::component_repository::ComponentRepository;
//! use crate::service::component_service::ComponentService;
//! use jira::models::core::IssueKey;
//! use jira::models::project::Component;
//! use std::sync::Arc;
//!
//! struct MockComponentRepository;
//!
//! impl ComponentRepository for MockComponentRepository {
//!     fn create_component(
//!         &self,
//!         _issue_key: &IssueKey,
//!         _components: &Vec<Component>
//!     ) -> Result<(), crate::error::WorklogError> {
//!         Ok(())
//!     }
//! }
//!
//! let repository = Arc::new(MockComponentRepository);
//! let service = ComponentService::new(repository);
//!
//! let issue_key = IssueKey::new("TEST-123".into());
//! let components = vec![
//!     Component::new("Backend".into()),
//!     Component::new("Frontend".into()),
//! ];
//!
//! match service.create_component(&issue_key, &components) {
//!     Ok(_) => println!("Components created successfully."),
//!     Err(e) => eprintln!("Failed to create components: {:?}", e),
//! }
//! ```
use crate::error::WorklogError;
use crate::repository::component_repository::ComponentRepository;
use jira::models::core::IssueKey;
use jira::models::project::Component;
use std::sync::Arc;

pub struct ComponentService {
    repository: Arc<dyn ComponentRepository>,
}

impl ComponentService {
    pub fn new(repository: Arc<dyn ComponentRepository>) -> Self {
        Self { repository }
    }

    /// Creates a new component for the given issue key.
    ///
    /// # Parameters
    ///
    /// * `issue_key` - A reference to the `IssueKey` representing the issue to which the components will be added.
    /// * `components` - A reference to a vector of `Component` instances that need to be created.
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the components are successfully created.
    /// * `Err(WorklogError)` if an error occurs while creating the components.
    pub fn create_component(
        &self,
        issue_key: &IssueKey,
        components: &Vec<Component>,
    ) -> Result<(), WorklogError> {
        self.repository.create_component(issue_key, components)
    }
}
