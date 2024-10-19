use secure_credentials::store_secure_token;

fn main() {
    let service = "com.autostoresystem.jira_worklog";
    let account = "steinar.cook@autostoresystem.com";
    let token = "my_secure_token";

    match store_secure_token(service, account, token) {
        Ok(_) => println!("Secure token stored successfully."),
        Err(e) => eprintln!("Failed to store secure token: {}", e),
    }
}
