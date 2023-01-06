use postgres;
use postgres::{Client, Error, Row};
use tokio_postgres::NoTls;
use crate::{Author, JiraIssue, Worklog, WorklogsPage};

pub async fn dbms_async_init() -> tokio_postgres::Client {
    let (client, connection) = tokio_postgres::connect(connect_str(), NoTls).await.unwrap();
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e);
        }
    });
    client
}

pub fn dbms_init() -> Client {
    match postgres::Client::connect(connect_str(), postgres::NoTls) {
        Ok(client) => client,
        Err(err) => panic!("Unable to connect to database: {:?}", err)
    }
}

pub fn connect_str() -> &'static str {
    "host=postgres.testenv.autostoresystem.com user=postgres password=uU7DP6WatYtUhEeNpKfq"
}

pub async fn insert_issue(dbms: &mut tokio_postgres::Client, issue: &JiraIssue) {
    let stmt = r#"insert into jira.issue (id, key) values($1,$2)
        on conflict
        do nothing
        "#;
    match dbms.execute(stmt, &[&issue.id, &issue.key]).await {
        Ok(_) => {}
        Err(e) => panic!("Unable to insert new issue {:?}, \nError: {:?}", &issue, e),
    }
}

pub async fn insert_author(dbms: &mut tokio_postgres::Client, author: &Author) -> i32 {
    let stmt = r#"insert into jira.author (account_id, email_address, display_name)
        values ($1,$2,$3)
        on conflict (account_id)
        do
            update
                set account_id = excluded.account_id
        returning id
        "#;

    match dbms.query_one(stmt, &[&author.accountId, &author.emailAddress,&author.displayName]).await {
        Ok(row) =>  row.get(0),
        Err(dbms_err) => panic!("Unable to insert new jira.author, reason: {:?}", dbms_err)
    }
}

pub async fn insert_worklog(dbms: &mut tokio_postgres::Client, account_id: &str, worklog: &Worklog)  {
    let stmt = r#"insert into jira.worklog (id, account_id, created,
            updated, started, timespent, timespentseconds, issueid)
        values ($1,$2,$3, $4, $5, $6, $7, $8)
        on conflict (id)
        do
            nothing
    "#;
    match dbms.execute(stmt, &[&worklog.id, &account_id, &worklog.created,
        &worklog.updated, &worklog.started, &worklog.timeSpent, &worklog.timeSpentSeconds,
        &worklog.issueId]).await {
        Ok(_) => (),
        Err(err) => panic!("Unable to upsert new worklog entry: {:?}", err)
    }
}

#[test]
fn test_insert_author() {

    let mut client = async {dbms_async_init().await};
    let author = Author {
        accountId: "a1".to_string(),
        emailAddress: Some("steinar@blabla.com".to_string()),
        displayName: "Steinar".to_string()
    };
    let id = insert_author(&mut client, &author);
    assert!(id > 0, "No value returned when inserting author");
    let id = insert_author(&mut client, &author);
    assert!(id > 0, "No value returned from second insert");

    client.execute("delete from jira.author where account_id=$1", &[&author.accountId]).unwrap();
}

#[test]
fn test_insert_worklog() {
    let mut client = dbms_init();
    let json = r#"{"startAt":0,"maxResults":1,"total":8884,"worklogs":[{"self":"https://autostore.atlassian.net/rest/api/2/issue/85002/worklog/129875","author":{"self":"https://autostore.atlassian.net/rest/api/2/user?accountId=557058%3A189520f0-d1fb-4a0d-b555-bc44ec1f4ebc","accountId":"557058:189520f0-d1fb-4a0d-b555-bc44ec1f4ebc","emailAddress":"borge.bekken@autostoresystem.com","avatarUrls":{"48x48":"https://secure.gravatar.com/avatar/0c67157f18660008baae96b0a2e40a61?d=https%3A%2F%2Favatar-management--avatars.us-west-2.prod.public.atl-paas.net%2Finitials%2FBB-1.png","24x24":"https://secure.gravatar.com/avatar/0c67157f18660008baae96b0a2e40a61?d=https%3A%2F%2Favatar-management--avatars.us-west-2.prod.public.atl-paas.net%2Finitials%2FBB-1.png","16x16":"https://secure.gravatar.com/avatar/0c67157f18660008baae96b0a2e40a61?d=https%3A%2F%2Favatar-management--avatars.us-west-2.prod.public.atl-paas.net%2Finitials%2FBB-1.png","32x32":"https://secure.gravatar.com/avatar/0c67157f18660008baae96b0a2e40a61?d=https%3A%2F%2Favatar-management--avatars.us-west-2.prod.public.atl-paas.net%2Finitials%2FBB-1.png"},"displayName":"Børge Bekken","active":true,"timeZone":"Europe/Oslo","accountType":"atlassian"},"updateAuthor":{"self":"https://autostore.atlassian.net/rest/api/2/user?accountId=557058%3A189520f0-d1fb-4a0d-b555-bc44ec1f4ebc","accountId":"557058:189520f0-d1fb-4a0d-b555-bc44ec1f4ebc","emailAddress":"borge.bekken@autostoresystem.com","avatarUrls":{"48x48":"https://secure.gravatar.com/avatar/0c67157f18660008baae96b0a2e40a61?d=https%3A%2F%2Favatar-management--avatars.us-west-2.prod.public.atl-paas.net%2Finitials%2FBB-1.png","24x24":"https://secure.gravatar.com/avatar/0c67157f18660008baae96b0a2e40a61?d=https%3A%2F%2Favatar-management--avatars.us-west-2.prod.public.atl-paas.net%2Finitials%2FBB-1.png","16x16":"https://secure.gravatar.com/avatar/0c67157f18660008baae96b0a2e40a61?d=https%3A%2F%2Favatar-management--avatars.us-west-2.prod.public.atl-paas.net%2Finitials%2FBB-1.png","32x32":"https://secure.gravatar.com/avatar/0c67157f18660008baae96b0a2e40a61?d=https%3A%2F%2Favatar-management--avatars.us-west-2.prod.public.atl-paas.net%2Finitials%2FBB-1.png"},"displayName":"Børge Bekken","active":true,"timeZone":"Europe/Oslo","accountType":"atlassian"},"created":"2022-02-04T16:22:28.554+0100","updated":"2022-02-04T16:22:44.384+0100","started":"2022-01-24T09:00:00.000+0100","timeSpent":"1d","timeSpentSeconds":27000,"id":"129875","issueId":"85002"}]}"#;

    let result = serde_json::from_str::<WorklogsPage>(&json).unwrap();
    let w = &result.worklogs[0];
    let a = &w.author;
    let _author_id = insert_author(&mut client, &a);

    insert_worklog(&mut client, &a.accountId, &w);

    // client.execute("delete from jira.worklog where id=$1", &[&w.id]);
}

#[test]
fn test_insert_issue() {
    let mut client = dbms_init();
    let issue = JiraIssue { id: "42".to_string(), self_url: "".to_string(), key: "XX-42".to_string(), worklogs: vec![] };
    insert_issue(&mut client, &issue);
}