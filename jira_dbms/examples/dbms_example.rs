fn main() {
    let _client = postgres::Client::connect("host=postgres.testenv.autostoresystem.com user=postgres password=uU7DP6WatYtUhEeNpKfq",postgres::NoTls).unwrap();

    let config = jira_lib::config::load_configuration().unwrap();
    let mut client = jira_dbms::dbms_init(&config.dbms.connect).unwrap();
    let row = client.query_one("select version()",&[]).unwrap();
    let version: &str = row.get(0);
    println!("Version: {version}");
}
