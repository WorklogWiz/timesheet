pub mod config;
pub mod error;

/// macOS Keychain integration for secure credential storage
#[cfg(target_os = "macos")]
pub mod macos;
