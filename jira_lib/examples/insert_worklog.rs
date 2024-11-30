use std::env;

#[tokio::main]
async fn main() {
    if let (Ok(host), Ok(user), Ok(token)) = (
        env::var("JIRA_HOST"),
        env::var("JIRA_USER"),
        env::var("JIRA_TOKEN"),
    ) {
        let jira_client = jira_lib::Jira::new(
            &format!("{host}/rest/api/latest"),
            &user,
            &token).expect("Error initializing jira client");
        let dt = chrono::offset::Local::now();

        println!("Executing ...");
        let _r = jira_client
            .insert_worklog("TIME-94", dt, 27000, "Rubbish comment")
            .await;
    }
}
