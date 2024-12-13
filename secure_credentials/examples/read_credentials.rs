#[cfg(target_os = "macos")]
use secure_credentials::macos::get_secure_token;

#[allow(unused_variables)]
fn main() {
    let service = "com.norns.timesheet";
    let account = "me@whereever.com";

    #[cfg(target_os = "macos")]
    match get_secure_token(service, account) {
        Ok(token) => println!("Retrieved secure token: {token}"),
        Err(e) => eprintln!("Failed to retrieve secure token: {e}"),
    }
}
