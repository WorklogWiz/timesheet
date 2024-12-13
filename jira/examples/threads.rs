use std::env;

use futures::{stream, Future, Stream, StreamExt};
use jira::{
    models::project::{JiraProjectsPage, Project},
    Credentials, Jira,
};
use lazy_static::lazy_static;
use reqwest::Client;
use tokio::time::Instant;

lazy_static! {
    static ref START_TIME: Instant = Instant::now();
}

#[tokio::main]
async fn main() {
    if let (Ok(host), Ok(user), Ok(token)) = (
        env::var("JIRA_HOST"),
        env::var("JIRA_USER"),
        env::var("JIRA_TOKEN"),
    ) {
        let http_client = Jira::new(&host, Credentials::Basic(user, token))
            .expect("Error initializing jira client")
            .client;

        let url = format!("https://{host}/rest/api/latest/project/search?maxResults=50&startAt=0");
        let urls = vec![url.clone(), url.clone(), url.clone(), url.clone()];

        let result = get_project_pages(&http_client, urls).await;
        let r2 = result.iter().flatten().collect::<Vec<&Project>>();

        println!(
            "Retrieved {} projects in {}ms",
            r2.len(),
            START_TIME.elapsed().as_millis()
        );
    } else {
        panic!("Missing env var JIRA_HOST, JIRA_USER or JIRA_TOKEN")
    }

    /*    for e in result {
            for p in e {
                println!("Project: {} {}", p.key, p.name);
            }
        }
    */
}

async fn get_project_pages(http_client: &Client, urls: Vec<String>) -> Vec<Vec<Project>> {
    get_project_futures_stream(http_client, urls)
        .buffer_unordered(10)
        .collect()
        .await
}

fn get_project_futures_stream(
    http_client: &Client,
    urls: Vec<String>,
) -> impl Stream<Item = impl Future<Output = Vec<Project>> + '_> + '_ {
    stream::iter(urls).map(|url| get_projects_from_page(http_client, url))
}

async fn get_projects_from_page(http_client: &Client, url: String) -> Vec<Project> {
    let start = Instant::now();
    let response = http_client.get(url.clone()).send().await.unwrap();
    let _elapsed = start.elapsed();

    // Downloads the entire body of the response and convert from JSON to type safe struct
    let typed_result = match response.status() {
        reqwest::StatusCode::OK => {
            // Transforms JSON in body to type safe struct
            match response.json::<JiraProjectsPage>().await {
                Ok(jira_projects_page) => jira_projects_page.values, // Everything OK, return the Worklogs struct
                Err(err) => panic!("ERROR Obtaining response in JSON format: {err:?}"),
            }
        }
        reqwest::StatusCode::UNAUTHORIZED => panic!("Not authorized, API key has probably changed"),
        reqwest::StatusCode::TOO_MANY_REQUESTS => {
            panic!("429 - Too many requests {:?}", response.headers())
        }

        other => {
            let decoded_url = urlencoding::decode(&url).unwrap();
            panic!(
                "Error code {:?} for {}\nencoded url={}",
                other, &decoded_url, &url
            );
        }
    };
    typed_result
}
