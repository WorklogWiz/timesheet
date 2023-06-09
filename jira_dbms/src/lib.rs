use postgres::Client;
use tokio_postgres::{Error, NoTls};
use jira_lib::{Author, JiraClient, JiraIssue, JiraProject, Worklog};
use std::fmt::Write;
use log::{debug, info};
use tokio_postgres::types::ToSql;
use std::collections::HashSet;
use std::process::exit;
use chrono::NaiveDateTime;
use reqwest::StatusCode;

const DBMS_CHUNK_SIZE: usize = 1000;

pub async fn dbms_async_init(connect: &str) -> Result<tokio_postgres::Client, Error > {

    debug!("Connecting with {}", connect);

    let result = tokio_postgres::connect(connect, NoTls).await;
    match result {
        Ok((client, connection)) => {

            tokio::spawn(async move {
                if let Err(e) = connection.await {
                    eprintln!("connection error: {}", e);
                }
            });
            Ok(client)
        }
        Err(err) => Err(err)
    }
}

pub fn dbms_init(connect: &str) -> Result<Client, Error> {
     Client::connect(connect, NoTls)
}

#[deprecated()]
pub fn connect_str() -> &'static str {
    "host=postgres.testenv.autostoresystem.com user=postgres password=uU7DP6WatYtUhEeNpKfq"
}

pub async fn insert_project(dbms: &mut tokio_postgres::Client, project: &JiraProject) {
    let stmt = r#"insert into
    jira.project  (id, key, name, url)
    values ($1, $2, $3, $4)
    on conflict (id) do nothing
    "#;

    match dbms.execute(stmt, &[&project.id, &project.key, &project.name, &project.url]).await {
        Ok(_) => {}
        Err(e) => panic!("Unable to insert project {:?}, cause: {}", project, e)
    }
}

pub async fn insert_issue(dbms: &mut tokio_postgres::Client, project_id: &str, issue: &JiraIssue) {
    let stmt = r#"
    with data(id, key, project_id, summary, asset_name) AS (
        values
            ($1, $2, $3, $4, $5)
    )
    insert into jira.issue (id, key, project_id, summary, asset_id)
            select data.id, data.key, data.project_id, data.summary, jira.asset.id
            from data left outer join jira.asset on data.asset_name = jira.asset.asset_name
        on conflict
        do nothing
        "#;
    let asset_name = issue.fields.asset.as_ref().map(|a| a.value.to_string());

    match dbms.execute(stmt, &[&issue.id, &issue.key, &project_id, &issue.fields.summary, &asset_name]).await {
        Ok(_) => {}
        Err(e) => panic!("Unable to insert new issue {:?}, \nError: {:?}", &issue, e),
    }
}

pub async fn insert_author(dbms: &mut tokio_postgres::Client, author: &Author) -> String {
    let stmt = r#"insert into jira.author (account_id, email_address, display_name)
        values ($1,$2,$3)
        on conflict (account_id)
        do
            update
                set account_id = excluded.account_id
        returning account_id
        "#;

    match dbms.query_one(stmt, &[&author.accountId, &author.emailAddress, &author.displayName]).await {
        Ok(row) => row.get(0),
        Err(dbms_err) => panic!("Unable to insert new jira.author, using sql: {}, \nreason: {:?}", stmt, dbms_err)
    }
}

pub async fn batch_insert_authors(dbms: &mut tokio_postgres::Client, authors: &[Author]) {
    for authors_chunk in authors.chunks(DBMS_CHUNK_SIZE) {
        let (sql, params) = compose_batch_insert_authors_sql(authors);
        match dbms.execute(sql.as_str(), &params[..]).await {
            Ok(_) => {}
            Err(err) => panic!("Unable to insert authors. Cause: {:?}", err)
        }
        info!("Upserted {} authors", authors_chunk.len());
    }
}

fn compose_batch_insert_authors_sql(authors_chucnk: &[Author]) -> (String, Vec<&(dyn ToSql + Sync)>) {
    let mut insert_stmt = r#"insert into
    jira.author (account_id, email_address, display_name)
        values
    "#.to_string();
    let on_conflict = "on conflict (account_id) do nothing";
    let mut params = Vec::<&(dyn ToSql + Sync)>::new();
    for (i, author) in authors_chucnk.iter().enumerate() {
        if i > 0 {
            write!(insert_stmt, ",").unwrap();
        }
        write!(insert_stmt, "\n{}", format_args!("( ${}, ${}, ${})", i * 3 + 1, i * 3 + 2, i * 3 + 3)).unwrap();
        params.push(&author.accountId);
        params.push(&author.emailAddress);
        params.push(&author.displayName);
    }
    write!(insert_stmt, " \n{}", on_conflict).unwrap();

    (insert_stmt, params)
}

pub async fn insert_worklog(dbms: &mut tokio_postgres::Client, worklog: &Worklog) {
    let stmt = r#"insert into jira.worklog (id, account_id, created,
            updated, started, timespent, timespentseconds, issueid)
        values ($1,$2,$3, $4, $5, $6, $7, $8)
        on conflict (id)
        do
            nothing
    "#;
    match dbms.execute(stmt, &[&worklog.id, &worklog.author.accountId, &worklog.created,
        &worklog.updated, &worklog.started, &worklog.timeSpent, &worklog.timeSpentSeconds,
        &worklog.issueId]).await {
        Ok(_) => (),
        Err(err) => panic!("Unable to upsert new worklog entry: {:?}", err)
    }
}


pub async fn batch_insert_worklogs(dbms: &mut tokio_postgres::Client, worklogs: &[Worklog]) {

    // Splits the insert statement into chunks of PostgresSQL limit of 1000 entries
    for worklog_chunck in worklogs.chunks(DBMS_CHUNK_SIZE) {
        let (sql, params) = compose_batch_insert_worklog_sql(worklog_chunck);
        match dbms.execute(sql.as_str(), &params[..]).await {
            Ok(_) => {}
            Err(e) => panic!("Failed to insert worklogs, reason {:?}", e),
        };
    }
}

/// Dynamically composes the SQL and the list of parameters to insert a batch of Worklog items
fn compose_batch_insert_worklog_sql(worklog_chunck: &[Worklog]) -> (String, Vec<&(dyn ToSql + Sync)>) {
    let mut insert_stmt = r#"
    insert into jira.worklog (
            id,
            account_id,
            created,
            updated,
            started,
            timespent,
            timespentseconds,
            issueid
           ) values
           "#.to_string();
    let on_conflict_part = String::from(" on conflict (id) do nothing ");

    let mut params = Vec::<&(dyn ToSql + Sync)>::new();
    for (i, worklog_entry) in worklog_chunck.iter().enumerate() {
        if i > 0 {
            write!(insert_stmt, ",").unwrap();
        }
        write!(insert_stmt, "\n{}", format_args!("( ${}, ${}, ${}, ${}, ${}, ${}, ${}, ${} )",
                                                 i * 8 + 1, i * 8 + 2, i * 8 + 3, i * 8 + 4, i * 8 + 5, i * 8 + 6, i * 8 + 7, i * 8 + 8)).unwrap();

        params.push(&worklog_entry.id);
        params.push(&worklog_entry.author.accountId);
        params.push(&worklog_entry.created);
        params.push(&worklog_entry.updated);
        params.push(&worklog_entry.started);
        params.push(&worklog_entry.timeSpent);
        params.push(&worklog_entry.timeSpentSeconds);
        params.push(&worklog_entry.issueId);
    }
    write!(insert_stmt, " \n{}", on_conflict_part).unwrap();

    (insert_stmt, params)
}

/// Extracts all issues and accompanying worklogs for the supplied list of projects. Worklogs are retrieved for work started after `startedAfter`, which
/// specified a timestamp in UNIX time, with a granularity of milliseconds
pub async fn etl_issues_worklogs_and_persist(jira_client: &JiraClient, dbms_client: &mut tokio_postgres::Client, projects: Vec<JiraProject>, issues_filter: Option<Vec<String>>, started_after: NaiveDateTime) {
    if projects.is_empty() {
        println!("No projects found!");
        return;
    }

    for (i, project) in projects.iter().enumerate() {
        info!("Project: {} {} {} {}", i, project.id, project.key, project.name);
    }

    info!("Retrieving the issues and worklogs ....");
    let filter = issues_filter.unwrap_or(vec![]);
    let jira_projects = match jira_client.get_issues_and_worklogs(projects, filter, started_after).await {
        Ok(r) => r,
        Err(e) => match e {
                sc => { eprintln!("get_issues_and_worklogs() failed with http code {}", e);
                exit(4);
            }
        }
    };
    info!("Tada: number of projects {}", jira_projects.len());

    info!("Collecting all authors from all worklog entries and making a unique list of them...");
    let mut authors = HashSet::new();
    for p in &jira_projects {
        for issue in &p.issues {
            for wlog in &issue.worklogs {
                authors.insert(
                    wlog.author.clone());
            }
        }
    }


    let mut unique_authors = Vec::from_iter(authors);
    unique_authors.sort_by(|a, b| a.accountId.cmp(&b.accountId));

    info!("Collecting the AutoStore project assets from each TIME issue");
    let project_assets: Vec<String> = extract_assets_from_time_issues(&jira_projects);

    debug!("Found these assets: {:?}", project_assets);
    insert_assets(dbms_client, &project_assets[..]).await;

    info!("Upserting {} authors", unique_authors.len());
    batch_insert_authors( dbms_client, &unique_authors[..]).await;


    for project in &jira_projects {
        println!("Project: {} {}", project.key, project.name);
        insert_project( dbms_client, project).await;

        for issue in &project.issues {
            insert_issue( dbms_client, &project.id, issue).await;
            if !issue.worklogs.is_empty() {
                println!("Processing {} worklogs for {}", issue.worklogs.len(), issue.key);
                batch_insert_worklogs( dbms_client, &issue.worklogs[..]).await;
            }
        }
    }
}

fn extract_assets_from_time_issues(projects: &[JiraProject]) -> Vec<String> {
    projects.iter()
        .flat_map(|p| p.issues.iter()
            .filter(|i| i.fields.asset.as_ref().is_some())
            .map(|i| i.fields.asset.as_ref().unwrap().value.to_string())).collect()
}


pub async fn insert_assets(dbms: &mut tokio_postgres::Client, assets: &[String]) {
    for asset_chunk in assets.chunks(DBMS_CHUNK_SIZE) {
        let mut sql = r#"insert into jira.asset (asset_name) values "#.to_string();
        let sql_on_conflict = "on conflict (asset_name) do nothing ";

        let mut params = Vec::<&(dyn ToSql + Sync)>::new();
        for (i, asset) in asset_chunk.iter().enumerate() {
            if i > 0 {
                write!(sql, ",").unwrap();
            }
            write!(sql, "\n{}", format_args!("( ${} ) ", i + 1)).unwrap();
            params.push(asset);
        }
        write!(sql, " {}", sql_on_conflict).unwrap();
        debug!("Executing {}", sql);

        match dbms.execute(&sql, &params[..]).await {
            Ok(_) => {}
            Err(e) => { panic!("Failed to insert authors, SQL: {} \n cause: {:?}", sql, e) }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jira_lib::{JiraAsset, JiraFields, WorklogsPage};

    async fn load_config_and_get_dbms_connection() -> tokio_postgres::Client {
        let config = jira_lib::config::load_configuration().unwrap();

        let mut client = dbms_async_init(&config.dbms.connect).await.unwrap();
        client
    }


    #[tokio::test]
    async fn test_insert_author() {
        let mut client = load_config_and_get_dbms_connection().await;
        let author = Author {
            accountId: "a1".to_string(),
            emailAddress: Some("steinar@blabla.com".to_string()),
            displayName: "Steinar".to_string(),
        };
        let account_id = insert_author(&mut client, &author).await;
        assert!(account_id.len() > 0, "No value returned when inserting author");
        let account_id = insert_author(&mut client, &author).await;
        assert!(account_id.len() > 0, "No value returned from second insert");

        client.execute("delete from jira.author where account_id=$1", &[&author.accountId]).await.unwrap();
    }


    #[tokio::test]
    async fn test_insert_worklog() {
        let mut client = load_config_and_get_dbms_connection().await;

        let json = r#"{"startAt":0,"maxResults":1,"total":8884,"worklogs":[{"self":"https://autostore.atlassian.net/rest/api/2/issue/85002/worklog/129875","author":{"self":"https://autostore.atlassian.net/rest/api/2/user?accountId=557058%3A189520f0-d1fb-4a0d-b555-bc44ec1f4ebc","accountId":"557058:189520f0-d1fb-4a0d-b555-bc44ec1f4ebc","emailAddress":"borge.bekken@autostoresystem.com","avatarUrls":{"48x48":"https://secure.gravatar.com/avatar/0c67157f18660008baae96b0a2e40a61?d=https%3A%2F%2Favatar-management--avatars.us-west-2.prod.public.atl-paas.net%2Finitials%2FBB-1.png","24x24":"https://secure.gravatar.com/avatar/0c67157f18660008baae96b0a2e40a61?d=https%3A%2F%2Favatar-management--avatars.us-west-2.prod.public.atl-paas.net%2Finitials%2FBB-1.png","16x16":"https://secure.gravatar.com/avatar/0c67157f18660008baae96b0a2e40a61?d=https%3A%2F%2Favatar-management--avatars.us-west-2.prod.public.atl-paas.net%2Finitials%2FBB-1.png","32x32":"https://secure.gravatar.com/avatar/0c67157f18660008baae96b0a2e40a61?d=https%3A%2F%2Favatar-management--avatars.us-west-2.prod.public.atl-paas.net%2Finitials%2FBB-1.png"},"displayName":"Børge Bekken","active":true,"timeZone":"Europe/Oslo","accountType":"atlassian"},"updateAuthor":{"self":"https://autostore.atlassian.net/rest/api/2/user?accountId=557058%3A189520f0-d1fb-4a0d-b555-bc44ec1f4ebc","accountId":"557058:189520f0-d1fb-4a0d-b555-bc44ec1f4ebc","emailAddress":"borge.bekken@autostoresystem.com","avatarUrls":{"48x48":"https://secure.gravatar.com/avatar/0c67157f18660008baae96b0a2e40a61?d=https%3A%2F%2Favatar-management--avatars.us-west-2.prod.public.atl-paas.net%2Finitials%2FBB-1.png","24x24":"https://secure.gravatar.com/avatar/0c67157f18660008baae96b0a2e40a61?d=https%3A%2F%2Favatar-management--avatars.us-west-2.prod.public.atl-paas.net%2Finitials%2FBB-1.png","16x16":"https://secure.gravatar.com/avatar/0c67157f18660008baae96b0a2e40a61?d=https%3A%2F%2Favatar-management--avatars.us-west-2.prod.public.atl-paas.net%2Finitials%2FBB-1.png","32x32":"https://secure.gravatar.com/avatar/0c67157f18660008baae96b0a2e40a61?d=https%3A%2F%2Favatar-management--avatars.us-west-2.prod.public.atl-paas.net%2Finitials%2FBB-1.png"},"displayName":"Børge Bekken","active":true,"timeZone":"Europe/Oslo","accountType":"atlassian"},"created":"2022-02-04T16:22:28.554+0100","updated":"2022-02-04T16:22:44.384+0100","started":"2022-01-24T09:00:00.000+0100","timeSpent":"1d","timeSpentSeconds":27000,"id":"129875","issueId":"85002"}]}"#;

        let result = serde_json::from_str::<WorklogsPage>(&json).unwrap();
        let w = &result.worklogs[0];
        let a = &w.author;
        let _author_id = insert_author(&mut client, &a).await;

        insert_worklog(&mut client, &w).await;

        // client.execute("delete from jira.worklog where id=$1", &[&w.id]);
    }

    #[test]
    fn test_compose_batch_insert_worklog_sql() {
        use std::fs;

        let contents = fs::read_to_string("tests/time-40_worklog_results.json").expect("Expected to load json file");
        let worklogs = serde_json::from_str::<WorklogsPage>(&contents).unwrap().worklogs;
        let (sql, _params) = compose_batch_insert_worklog_sql(&worklogs[0..13]);
        println!("SQL: {}", sql);
    }

    #[tokio::test]
    async fn test_insert_assets() {
        let mut dbms = load_config_and_get_dbms_connection().await;

        let assets = [
            "Project_Pointer Light".to_string(),
            "Project_Design Tools Redesign".to_string(),
            "Project_ASConnect".to_string(),
            "Project_Unify Analytics 2.0".to_string(),
            "Project_Service Journal".to_string(),
            "Project_ Status Code Lookup".to_string(),
            "Project_Interface http Modernization".to_string(),
            "Project_Console 2.0 Redesign".to_string(),
            "Project_WMS SDK Rework".to_string(),
            "Project_WMS Emulator".to_string(),
        ];
        insert_assets(&mut dbms, &assets).await;
    }

    #[tokio::test]
    async fn test_insert_issues() {
        let mut dbms = load_config_and_get_dbms_connection().await;

        let project_id: String = match dbms.query_one("select jira.project.id from jira.project limit 1", &[]).await {
            Ok(row) => row.get(0),
            Err(err) => panic!("Unable to retrieve a prject id from DBMS {:?}", err)
        };
        let asset_id: i32 = match dbms.query_one("select id from jira.asset where asset_name='Project_Interface http Modernization'", &[]).await {
            Ok(row) => row.get(0),
            Err(err) => panic!("Unable to retrive asset_id, cause: {:?}", err)
        };

        let issue = JiraIssue {
            id: "42".to_string(),
            self_url: "www.rubbish.com".to_string(),
            key: "SOC-42".to_string(),
            worklogs: vec![],
            fields: JiraFields {
                summary: "SOC-42 is just an example".to_string(),
                asset: Some(
                    JiraAsset {
                        id: asset_id.to_string(),
                        value: "Project_Interface http Modernization".to_string(), url: "rubbish".to_string()
                    }
                ),
            },
        };

        insert_issue(&mut dbms, &project_id, &issue).await;
        let asset_id: Option::<i32> = match dbms.query_one("select asset_id from jira.issue where issue.id=$1", &[&issue.id]).await {
            Ok(row) => row.get(0),
            Err(err) => panic!("Unable to retrieve the asset_id from inserted issue. Cause: {:?}", err)
        };
        assert!(asset_id.is_some(),"Ouch unable to retrieve the asset back from the database");

        let result = match dbms.execute("delete from jira.issue where id=$1", &[&issue.id]).await {
            Ok(rows) => rows,
            Err(err) => panic!("Unable to remove inserted test data {:?}", err),
        };
        assert_eq!(result, 1);
    }
}