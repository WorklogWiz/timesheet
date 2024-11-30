use std::env;

use futures::StreamExt;

use jira::{Credentials, Jira};
use jira::models::worklog::WorklogsPage;

#[tokio::main]
async fn main() {
    // Creates HTTP client with all the required credentials
    if let (Ok(host), Ok(user), Ok(token)) = (
        env::var("JIRA_HOST"),
        env::var("JIRA_USER"),
        env::var("JIRA_TOKEN"),
    ) {
        let jira_client = Jira::new(&host, Credentials::Basic(user, token))
            .expect("Error initializing jira client");

        let entries = vec!["TIME-12", "TIME-5"];

        let bodies = futures::stream::iter(entries)
            .map(|issue| {
                let client = jira_client.client.clone();
                let host = host.clone();
                tokio::spawn(async move {
                    let resource = format!("/issue/{issue}/worklog");
                    let url = format!("{host}{resource}");
                    println!("http get {url}");
                    let response = client.get(url).send().await.unwrap();

                    // Downloads the entire body of the response and convert from JSON to type safe struct
                    let typed_result: WorklogsPage = match response.status() {
                        reqwest::StatusCode::OK => {
                            // Transforms JSON in body to type safe struct
                            match response.json::<WorklogsPage>().await {
                                Ok(wl) => wl, // Everything OK, return the Worklogs struct
                                Err(err) => {
                                    panic!("ERROR Obtaining response in JSON format: {err:?}")
                                }
                            }
                        }
                        reqwest::StatusCode::UNAUTHORIZED => {
                            panic!("Not authorized, API key has probably changed")
                        }
                        other => {
                            panic!("Something unexpected happened {other:?}");
                        }
                    };
                    typed_result
                })
            })
            .buffer_unordered(2);

        bodies
            .for_each(|b| async {
                match b {
                    Ok(w) => println!("-- {w:?}"),
                    Err(e) => eprintln!("Ouch, a real error {e:?}"),
                }
            })
            .await;
    } else {
        panic!("Missing env var JIRA_HOST, JIRA_USER or JIRA_TOKEN")
    }
}
