/// The `Cleanup` trait defines an interface for types that need to release resources
/// or perform cleanup operations when they're no longer needed.
///
/// Implementations should ensure that:
/// - All resources are properly released
/// - The cleanup operation is idempotent (can be called multiple times safely)
/// - Partial cleanup doesn't leave the system in an invalid state
#[async_trait::async_trait]
#[allow(dead_code)]
pub trait TestCleanup {
    /// Performs cleanup operations for this type.
    ///
    /// # Errors
    ///
    /// Errors are silently ignored.
    ///
    /// # Cancellation Safety
    ///
    /// Implementations should ensure that this method is cancellation-safe,
    /// and that partial cleanup doesn't leave resources in an invalid state.
    async fn cleanup(&mut self);
}
