use std::io;
use chrono::Utc;
use jira_lib::{Author, Worklog};

fn main() {
    let mut csv_writer = csv::WriterBuilder::new()
        .delimiter(b';')
        .from_writer(io::stdout());
    csv_writer.serialize(
         Author {
            accountId: "account 42".to_string(),
            displayName: "Steinar".to_string(),
            emailAddress: Some("steinar@cook.no".to_string()),
    }).expect("Serializing failed!");
    // csv_writer.flush().expect("Flush failed!");

}