use lazy_static::lazy_static;
use postgres::Client;
use jira::dbms;

lazy_static! {
    static ref DBMS: Client = dbms::init();
}

#[test]
fn test_insert_author() {

    let stmt = r#"insert into jira.author (account_id, email_address, display_name)
        values ($1,$2,$3)
        on conflict (account_id)
        do nothing"#;

}