#[cfg(target_os = "macos")]
use secure_credentials::macos::get_secure_token;

#[allow(unused_variables)]
fn main() {
    let service = "com.autostoresystem.jira_worklog";
    let account = "steinar.cook@autostoresystem.com";

    #[cfg(target_os = "macos")]
    match get_secure_token(service, account) {
        Ok(token) => println!("Retrieved secure token: {token}"),
        Err(e) => eprintln!("Failed to retrieve secure token: {e}"),
    }
}
