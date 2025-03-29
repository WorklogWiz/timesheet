#[cfg(target_os = "macos")]
use secure_credentials::macos::store_secure_token;

#[allow(unused_variables)]
fn main() {
    let service = "com.norn.timesheet";
    let account = "me@whereever.com";
    let token = "my_secure_token";

    #[cfg(target_os = "macos")]
    match store_secure_token(service, account, token) {
        Ok(()) => println!("Secure token stored successfully."),
        Err(e) => eprintln!("Failed to store secure token: {e}"),
    }
}
