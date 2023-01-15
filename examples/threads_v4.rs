use chrono::Datelike;
use env_logger::Env;
use futures::{Future, stream, Stream, StreamExt};
use lazy_static::lazy_static;
use log::info;
use reqwest::Client;
use tokio::time::Instant;
use jira::{get_jira_data_from_url, JiraProject, JiraProjectsPage, Worklog};

lazy_static! {
    static ref START_TIME: Instant = Instant::now();
}

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("debug")).init();

    let http_client = jira::http_client();

    let start = Instant::now();
    let projects = jira::get_all_projects(&http_client).await;
    let elapsed = start.elapsed().as_millis();

    info!("Retrieved {} projects via all_jira_projects in {}ms", &projects.len(), elapsed);



}


async fn all_jira_projects(http_client: &Client) -> Vec<JiraProject> {
    let first_page = get_first_project_page(&http_client).await;
    let subsequent_pages = get_subsequent_project_pages(&http_client, &first_page).await;

    let mut result = first_page.values;
    for project_page in subsequent_pages.into_iter() {
        result.append(&mut project_page.values.into_iter().filter(|p| { !p.is_private}).collect());
    }
    result
}


async fn get_first_project_page(http_client: &Client) -> JiraProjectsPage {
    get_jira_data_from_url::<JiraProjectsPage>(&http_client, jira::compose_project_url(0, 1024)).await
}

async fn get_subsequent_project_pages(http_client: &Client, first_page: &JiraProjectsPage) -> Vec<JiraProjectsPage> {
    let urls = jira::compose_project_urls(first_page.startAt + first_page.maxResults, first_page.maxResults, first_page.total.unwrap());

    get_project_futures_stream(http_client, urls).buffer_unordered(10).collect().await
}

fn get_project_futures_stream(http_client: &Client, urls: Vec<String>) -> impl Stream<Item=impl Future<Output=JiraProjectsPage> + '_> + '_ {
    stream::iter(urls).map(|url| {
        get_jira_data_from_url::<JiraProjectsPage>(http_client, url)
    })
}




