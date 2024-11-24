#[cfg(target_os = "macos")]
use security_framework::os::macos::keychain::SecKeychain;
#[cfg(target_os = "macos")]
use security_framework::os::macos::passwords::find_generic_password;
use std::error::Error;

#[cfg(target_os = "macos")]
/// Store the `token` into the macos Keychain for the provided `service` with the user account
/// identified by `account`
///
/// # Errors
///
/// Will return `Err` if writing to the macos keychain failed for some reason
pub fn store_secure_token(service: &str, account: &str, token: &str) -> Result<(), Box<dyn Error>> {
    // Add a generic password to the keychain
    SecKeychain::default()?.set_generic_password(
        service,          // Service name (e.g., website or application identifier)
        account,          // Account name (e.g., username or email)
        token.as_bytes(), // Password or token data
    )?;

    Ok(())
}

#[cfg(target_os = "macos")]
/// Retrieves the secure token associated with `service` and `account`
///
/// # Errors
///
/// Returns `Err` if the secure token could not be obtained from the keychain
///
pub fn get_secure_token(service: &str, account: &str) -> Result<String, Box<dyn Error>> {
    // Find the generic password in the keychain
    let password = find_generic_password(None, service, account)?;

    // Convert the password to a string
    let password_str = String::from_utf8(password.0.to_vec())?;

    Ok(password_str)
}
