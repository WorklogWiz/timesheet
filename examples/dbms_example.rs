use postgres;

fn main() {
    let mut client = postgres::Client::connect("host=postgres.testenv.autostoresystem.com user=postgres password=uU7DP6WatYtUhEeNpKfq",postgres::NoTls).unwrap();

    let row = client.query_one("select version()",&[]).unwrap();
    let version: &str = row.get(0);
        println!("Version: {}", version);
}