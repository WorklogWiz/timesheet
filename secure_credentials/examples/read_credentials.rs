use secure_credentials::get_secure_token;

fn main() {
    let service = "com.autostoresystem.jira_worklog";
    let account = "steinar.cook@autostoresystem.com";

    match get_secure_token(service, account) {
        Ok(token) => println!("Retrieved secure token: {}", token),
        Err(e) => eprintln!("Failed to retrieve secure token: {}", e),
    }
}
