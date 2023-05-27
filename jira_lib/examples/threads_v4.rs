use env_logger::Env;
use futures::{Future, stream, Stream, StreamExt};
use lazy_static::lazy_static;
use log::info;
use reqwest::Client;
use tokio::time::Instant;
use jira_lib::{get_issues_for_single_project, get_jira_data_from_url, JiraIssue, JiraProject, JiraProjectsPage};

lazy_static! {
    static ref START_TIME: Instant = Instant::now();
}

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("debug")).init();

    let http_client = jira_lib::http_client();

    let start = Instant::now();
    let projects = jira_lib::get_all_projects(&http_client, vec![]).await;
    let elapsed = start.elapsed().as_millis();

    info!("Retrieved {} projects via all_jira_projects in {}ms", &projects.len(), elapsed);

    let project_keys = projects.iter().map(|p| p.key.to_owned()).collect();

    let _issues = get_all_the_bloody_issues(&http_client, project_keys).await;


}


async fn _all_jira_projects(http_client: &Client) -> Vec<JiraProject> {
    let first_page = get_first_project_page(&http_client).await;
    let subsequent_pages = get_subsequent_project_pages(&http_client, &first_page).await;

    let mut result = first_page.values;
    for project_page in subsequent_pages.into_iter() {
        result.append(&mut project_page.values.into_iter().filter(|p| { !p.is_private}).collect());
    }
    result
}

#[allow(dead_code)]
async fn get_first_project_page(http_client: &Client) -> JiraProjectsPage {
    get_jira_data_from_url::<JiraProjectsPage>(&http_client, jira_lib::compose_project_url(0, 1024)).await
}

#[allow(dead_code)]
async fn get_subsequent_project_pages(http_client: &Client, first_page: &JiraProjectsPage) -> Vec<JiraProjectsPage> {
    let urls = jira_lib::compose_project_urls(first_page.startAt + first_page.maxResults, first_page.maxResults, first_page.total.unwrap());

    get_project_futures_stream(http_client, urls).buffer_unordered(10).collect().await
}

fn get_project_futures_stream(http_client: &Client, urls: Vec<String>) -> impl Stream<Item=impl Future<Output=JiraProjectsPage> + '_> + '_ {
    stream::iter(urls).map(|url| {
        get_jira_data_from_url::<JiraProjectsPage>(http_client, url)
    })
}

async fn get_all_the_bloody_issues(http_client: &Client, project_keys : Vec<String> ) -> Vec<JiraIssue> {

    let futures_result: Vec<Vec<JiraIssue>> = stream::iter(project_keys).map(|p| {

        get_issues_for_single_project(http_client, p)
    }).buffer_unordered(10).collect().await;

    futures_result.into_iter().flatten().collect()
}

fn _get_issues_futures_stream(http_client: &Client, projects: Vec<String>) -> impl Stream<Item=impl Future<Output=Vec<JiraIssue>> + '_> + '_ {
    stream::iter(projects).map(|p| {

        get_issues_for_single_project(http_client, p)
    })
}



