use std::sync::Arc;
use worklog::{error::WorklogError, ApplicationRuntime, ApplicationRuntimeBuilder};

/// Creates a test runtime with a temporary database
pub fn create_test_runtime() -> Result<Arc<ApplicationRuntime>, WorklogError> {
    // Initialize the runtime with this database in memory
    let runtime = ApplicationRuntimeBuilder::new()
        .use_in_memory_db()
        .use_jira_test_instance()
        .build()?;

    // Return the runtime
    Ok(Arc::new(runtime))
}
