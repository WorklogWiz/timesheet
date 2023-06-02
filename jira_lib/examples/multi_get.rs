
use futures::{ StreamExt};
use tokio;

use jira_lib::WorklogsPage;

#[tokio::main]
async fn main() {
    // Creates HTTP client with all the required credentials
    let jira_client = jira_lib::create_jira_client();


    let entries = vec!["TIME-12", "TIME-5"];

    let bodies = futures::stream::iter(entries)
        .map(|issue| {
            let client = jira_client.http_client.clone();
            tokio::spawn(
                async move {

                let resource = format!("/issue/{}/worklog", issue);
                let url = format!("{}{}", jira_lib::JIRA_URL, resource);
                println!("http get {}", url);
                let response = client
                    .get(url)
                    .send()
                    .await
                    .unwrap();

                // Downloads the entire body of the response and convert from JSON to type safe struct
                let typed_result: WorklogsPage = match response.status() {
                    reqwest::StatusCode::OK => {
                        // Transforms JSON in body to type safe struct
                        match response.json::<WorklogsPage>().await {
                            Ok(wl) => wl, // Everything OK, return the Worklogs struct
                            Err(err) => panic!("EROR Obtaining response in JSON format: {:?}", err)
                        }
                    }
                    reqwest::StatusCode::UNAUTHORIZED => panic!("Not authorized, API key has probably changed"),
                    other => {
                        panic!("Something unexpected happened {:?}", other);
                    }
                };
                typed_result
            })
        }).buffer_unordered(2);

    bodies.for_each(|b| async {
        match b {
            Ok(w) => println!("-- {:?}", w),
            Err(e) => eprintln!("Ouch, a real error {:?}",e),
        }
    })
        .await;
}