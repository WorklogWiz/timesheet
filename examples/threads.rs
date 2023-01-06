use futures::{stream, Future, Stream, StreamExt};
use lazy_static::lazy_static;
use rand::distributions::{Distribution, Uniform};
use std::time::Duration;
use reqwest::Client;
use tokio::time::{sleep, Instant};
use jira::{JiraProject, JiraIssue, JiraProjectsPage};

lazy_static! {
    static ref START_TIME: Instant = Instant::now();
}

#[tokio::main]
async fn main() {
    let  http_client = jira::http_client();

    let urls = vec![
        "https://autostore.atlassian.net/rest/api/latest/project/search?maxResults=50&startAt=0".to_string(),
        "https://autostore.atlassian.net/rest/api/latest/project/search?maxResults=50&startAt=50".to_string(),
        "https://autostore.atlassian.net/rest/api/latest/project/search?maxResults=50&startAt=100".to_string(),
        "https://autostore.atlassian.net/rest/api/latest/project/search?maxResults=50&startAt=150".to_string()];

    let result = get_project_pages(&http_client, urls).await;
    let r2 = result.iter().flatten().collect::<Vec<&JiraProject>>();

    println!("Retrieved {} projects in {}ms", r2.len(), START_TIME.elapsed().as_millis());

/*    for e in result {
        for p in e {
            println!("Project: {} {}", p.key, p.name);
        }
    }
*/}

async fn get_project_pages(http_client: &Client ,urls: Vec<String>) -> Vec<Vec<JiraProject>> {
    get_project_futures_stream(http_client, urls).buffer_unordered(10).collect().await
}


fn get_project_futures_stream(http_client: &Client, urls: Vec<String>) -> impl Stream<Item = impl Future<Output = Vec<JiraProject>> +'_> + '_ {
    stream::iter(urls).map(|url| get_projects_from_page(http_client, url))
}


async fn get_projects_from_page(http_client: &Client, url: String) -> Vec<JiraProject> {
    let start = Instant::now();
    let response = http_client.get(url.clone()).send().await.unwrap();
    let elapsed = start.elapsed();

    // Downloads the entire body of the response and convert from JSON to type safe struct
    let typed_result = match response.status() {
        reqwest::StatusCode::OK => {
            // Transforms JSON in body to type safe struct
            match response.json::<JiraProjectsPage>().await {
                Ok(jira_projects_page) => jira_projects_page.values, // Everything OK, return the Worklogs struct
                Err(err) => panic!("EROR Obtaining response in JSON format: {:?}", err),
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
