use std::collections::HashSet;
use jira::{Author, get_issues_and_worklogs, http_client};
use log;
use log::{info,};

use clap::Parser;
use jira::dbms::{dbms_async_init, dbms_init, insert_author, insert_issue, insert_worklog};

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
    started_after: Option<String>,
    /// Limits worklogs extraction to these projects. Defaults to "all"
    ///
    /// Example: TIME RGA A3SRS
    #[clap(short, long)]
    projects : Option<Vec<String>>,
    /// Limits worklogs extraction the list of issues supplied. Defaults to "all"
    ///
    /// Excludes the "projects" argument.
    #[clap(short, long, conflicts_with="projects")]
    issues: Option<Vec<String>>,
    /// Filters worklog entries for given user (email)
    ///
    /// User is identified with email address, i.e. steinar.cook@autostoresystem.com
    #[clap(short, long)]
    users: Option<Vec<String>>,
}

#[tokio::main]
async fn main() {
    env_logger::init();
    let args = Cli::parse();

    info!("Starting up");

    if let Some(projects) = args.projects.as_deref() {
        println!("Extracting from projects: {:?}", projects);
    } else {
        println!("Extracting all projects");
    }

    // Creates HTTP client with all the required credentials
    let http_client = http_client();

    let projects = jira::get_projects_filtered(&http_client, args.projects).await;

    if projects.is_empty() {
        println!("No projects found!");
        return ();
    }

    for (i, project) in projects.iter().enumerate() {
        println!("Project: {} {} {} {}", i, project.id, project.key, project.name);
    }

    println!("Retrieving the issues and worklogs ....");
    let results = get_issues_and_worklogs(&http_client, projects).await;
    println!("Tada: number of projects {}", results.len());

    println!("Spinning over the authors...");
    let mut authors = HashSet::new();
    for p in &results {
        for issue in &p.issues {
            println!("Issue {}", issue.key);
            for wlog in &issue.worklogs {
                print!("\tAdding author {}", &wlog.author.displayName);
                authors.insert(wlog.author.clone());
                println!("..done");
            }
        }
    }
    println!("Connecting to DBMS...");
    let mut client = dbms_async_init().await;

    println!("Inserting authors....");
    for a in authors {
        println!("Adding author {}", a.displayName);
        insert_author(&mut client, &a).await;
    }

    for p in &results {
        println!("Project: {} {}", p.key, p.name);
        for issue in &p.issues {
            println!("\t{}", issue.key);
            insert_issue(&mut client, &issue).await;
            for wlog in &issue.worklogs {
                println!("\t\t{} - {}", wlog.author.displayName, wlog.timeSpent);
                insert_worklog(&mut client, &wlog.author.accountId, &wlog).await;
            }
        }
    }
}

#[test]
fn verify_cli() {
    use clap::CommandFactory;
    Cli::command().debug_assert()
}