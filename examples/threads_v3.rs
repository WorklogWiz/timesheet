use chrono::Datelike;
use env_logger::Env;
use futures::{Future, stream, Stream, StreamExt};
use lazy_static::lazy_static;
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
    let result = all_jira_projects(&http_client).await;
    let elapsed1 = start.elapsed().as_millis();

    println!("Retrieved {} projects in {}ms", result.len(), elapsed1);
/*    for (i,project) in result.iter().enumerate() {
        println!("{:>3} {:5} {}", i, project.key, project.name);
    }
*/
    let start = Instant::now();
    let _result = jira::get_all_projects(&http_client).await;
    let elapsed = start.elapsed().as_millis();

    println!("all_jira_projects: {}, get_all_projects: {}", elapsed1, elapsed);

    for issue in vec!["TIME-39","RGA-8", "TIME-40"] {
        let worklogs = jira::get_worklogs_for(&http_client, issue.to_string(), ).await;
        let result : Vec<Worklog> = worklogs.into_iter().filter(|w| w.author.displayName == "Steinar Overbeck Cook").collect();

        println!("Issue: {issue}, worklogs: {}", result.len());
        let mut seconds = 0;
        for w in result {
            println!("{}; {};{};{}", w.started.date_naive(),  w.timeSpent, w.timeSpentSeconds, w.timeSpentSeconds as f64 / 3600.0);
            seconds += w.timeSpentSeconds;
        }
        println!("-----------------");
        println!("Total {} {}", seconds, seconds as f64 / 3600.0);
    }

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




