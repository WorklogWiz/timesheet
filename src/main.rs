use chrono::{Duration, FixedOffset, NaiveDate, NaiveDateTime};
use jira::{dbms, get_worklogs_for, http_client, midnight_a_month_ago_in, Worklog};
use log;
use log::info;

use clap::Parser;
use env_logger::Env;
use futures::{Future, stream, Stream, StreamExt};
use reqwest::Client;
use jira::dbms::etl_issues_worklogs_and_persist;

#[derive(Parser, Default, Debug)]
#[clap(author, version, about)]
/// Extracts all Jira worklogs for all issues for all projects not marked as private.
///
/// Extracts all the non-private projects after which all issues, which have not been "Resolved",
/// are retrieved using this JQL: `project=<proj_key> and resolution=Unresolved`.
/// Finally the worklogs for each issue is retrieved going back 12 months from the
/// current date.
struct Cli {
    /// Date in ISO8601 format (YYYY-MM-DD) from which point worklogs should be retrieved
    #[clap(short, long)]
    after: Option<String>,
    /// Retrieve worklog entries before date in ISO8601 format (YYYY-MM-DD)
    #[clap(short, long)]
    before: Option<String>,
    /// Limits worklogs extraction to these projects. Defaults to "all"
    ///
    /// Example: TIME RGA A3SRS
    #[clap(short, long)]
    projects : Option<Vec<String>>,
    /// Limits worklogs extraction the list of issues supplied. Defaults to "all"
    ///
    /// Excludes the "projects" argument.
    #[clap(short, long)]
    issues: Option<Vec<String>>,
    /// Filters worklog entries for given user (email)
    ///
    /// User is identified with email address, i.e. steinar.cook@autostoresystem.com
    #[clap(short, long)]
    users: Option<Vec<String>>,
}

#[tokio::main]
async fn main() {
    // RUST_LOG
    env_logger::Builder::from_env(Env::default().default_filter_or("debug")).init();

    let args = Cli::parse();

    info!("Starting up ...");

    // Creates HTTP client with all the required credentials
    let http_client = http_client();

    let startedAfter = match args.after.as_deref() {
        Some(after_spec) => {
            NaiveDateTime::parse_from_str((after_spec.to_string() + "T00:00").as_str(), "%Y-%m-%dT%H:%M").unwrap()
        },
        None => {
            midnight_a_month_ago_in()
        }
    };

    println!("Retrieving worklogs after {}",startedAfter.to_string());

    match args {
        Cli { projects: None, issues: None,users,..} => process_all_projects(&http_client, users,startedAfter).await,
        Cli { projects, issues: None, users, .. } => process_project_worklogs_filtered(&http_client, projects, users, startedAfter).await,
        // This no longer gives meaning, you need to extract project info before issues
        // Cli { projects: None, issues, users, ..} => process_issue_worklogs(&http_client, issues.unwrap(), users).await,
        Cli { projects, issues, users, .. } => process_project_issues   (&http_client, projects, issues,startedAfter).await,

        _ => unimplemented!("This combination of args is unknown"),
    }
}

async fn process_project_issues(http_client: &Client, projects: Option<Vec<String>>, issues: Option<Vec<String>>, startedAfter: NaiveDateTime) {
    let projects = jira::get_projects_filtered(http_client, projects).await;
    etl_issues_worklogs_and_persist(http_client, projects, issues, startedAfter).await;
}

/// Deprecated, left to illustrate alternativ usage of futures
async fn _process_issue_worklogs(http_client: &Client, issues: Vec<String>, _users: Option<Vec<String>>) {
    let worklogs = execute_worklogs_futures(http_client, issues).await;
    println!("Found {} worklog entries", worklogs.len());
}

async fn execute_worklogs_futures(http_client: &Client, issues: Vec<String>) -> Vec<Worklog> {
    let result: Vec<Vec<Worklog>> = worklogs_stream(http_client, issues).buffer_unordered(10).collect().await;
    result.into_iter().flatten().collect()
}

fn worklogs_stream(http_client: &Client, issues: Vec<String>) -> impl Stream<Item=impl Future<Output=Vec<Worklog>> + '_> + '_{
    stream::iter(issues).map(move |issue| {
        get_worklogs_for(&http_client, issue,midnight_a_month_ago_in() )
    })
}

async fn process_project_worklogs_filtered(http_client: &Client, projects: Option<Vec<String>>, _users: Option<Vec<String>>, startedAfter: NaiveDateTime) {
    let projects = jira::get_projects_filtered(http_client, projects).await;
    dbms::etl_issues_worklogs_and_persist(http_client, projects, None, startedAfter).await;
}

async fn process_all_projects(http_client: &Client, _users: Option<Vec<String>>, started_after: NaiveDateTime) {
    println!("Extracting all projects, filtering on users {:?}", _users);

    let projects = jira::get_all_projects(&http_client,vec![]).await;

    dbms::etl_issues_worklogs_and_persist(http_client, projects, None, started_after).await;
}

#[test]
fn verify_cli() {
    use clap::CommandFactory;
    Cli::command().debug_assert()
}