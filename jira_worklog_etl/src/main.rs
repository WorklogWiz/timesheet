use chrono::{NaiveDateTime};
use jira_lib::{ JiraClient,  midnight_a_month_ago_in};
use log::info;
use clap::Parser;
use env_logger::Env;
use jira_dbms::etl_issues_worklogs_and_persist;

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
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    let args = Cli::parse();    // Parses the command line

    info!("Starting up ...");

    let configuration = match jira_lib::config::load_configuration() {
        Ok(c) => c,
        Err(e) => panic!("Unable to load configuration from {}, cause:{}", jira_lib::config::config_file_name().to_string_lossy(),e)
    };

    let jira_client = match jira_lib::JiraClient::new(&configuration.jira.jira_url, &configuration.jira.user, &configuration.jira.token){
        Ok(c) => c,
        Err(e) => panic!("Unable to create Jira http client: {}", e)
    };

    let mut dbms_client: tokio_postgres::Client  =  match jira_dbms::dbms_async_init(&configuration.dbms.connect).await {
        Ok(dbms) => dbms,
        Err(e) => panic!("Unable to connect to the database: {}. \nHave you started VPN?", e)
    };

    for row in dbms_client.query("select version()",&[]).await.unwrap(){
        let s: String = row.get(0);
        println!("DBMS is {}", s);
    }

    let started_after = match args.after.as_deref() {
        Some(after_spec) => {
            NaiveDateTime::parse_from_str((after_spec.to_string() + "T00:00").as_str(), "%Y-%m-%dT%H:%M").unwrap()
        },
        None => {
            midnight_a_month_ago_in()
        }
    };

    println!("Retrieving worklogs after {}", started_after);

    match args {
        Cli { projects: None, issues: None,users,..} => process_all_projects(&jira_client, &mut dbms_client, users, started_after).await,
        Cli { projects, issues: None, users, .. } => process_project_worklogs_filtered(&jira_client, &mut dbms_client, projects, users, started_after).await,
        Cli { projects, issues, users, .. } => process_project_issues(&jira_client, &mut dbms_client, projects, issues, started_after, users).await,
    }
}

async fn process_project_issues(jira_client: &JiraClient, dbms_client: &mut tokio_postgres::Client, projects: Option<Vec<String>>, issues: Option<Vec<String>>, started_after: NaiveDateTime, _users: Option<Vec<String>>) {
    let projects = jira_client.get_projects_filtered(projects).await;
    etl_issues_worklogs_and_persist(jira_client, dbms_client,projects, issues, started_after).await;
}

async fn process_project_worklogs_filtered(jira_client: &JiraClient,  dbms_client: &mut tokio_postgres::Client, projects: Option<Vec<String>>, _users: Option<Vec<String>>, started_after: NaiveDateTime) {
    let projects = jira_client.get_projects_filtered(projects).await;
    etl_issues_worklogs_and_persist(jira_client, dbms_client, projects, None, started_after).await;
}

async fn process_all_projects(jira_client: &JiraClient, dbms_client: &mut tokio_postgres::Client, _users: Option<Vec<String>>, started_after: NaiveDateTime) {
    println!("Extracting all projects, filtering on users {:?}", _users);

    let projects = jira_client.get_all_projects(vec![]).await;

    etl_issues_worklogs_and_persist(jira_client, dbms_client, projects, None, started_after).await;
}

#[test]
fn verify_cli() {
    use clap::CommandFactory;
    Cli::command().debug_assert()
}