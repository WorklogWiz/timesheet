
extern crate core;

use jira;
use jira::WorklogsPage;
use jira::Worklog;

use chrono::{Datelike, DateTime, NaiveDate, Timelike, Utc};

use std::fs;
use serde_json::Value;

#[test]
fn test_deserialze() {
    let contents = fs::read_to_string("tests/time-40_worklog_results.json").expect("Expected to load json file");
    let json: serde_json::Value = serde_json::from_str(&contents).expect("Json is not correctly formatted.");
    assert_eq!(json["maxResults"],5000,"Invalid results found in start of Json");
    let worklogs = &json["worklogs"];

    match &worklogs {
        Value::Array(entries) => assert!(entries.len() > 10, "Worklogs does not contain more than 10 entries"),
        _  => panic!("json object 'worklogs' is incorrect type"),
    }
}

#[test]
fn test_parse_worklog() {
    let contents = fs::read_to_string("tests/time-40_worklog_results.json").expect("Expected to load json file");
    let log: WorklogsPage = serde_json::from_str(&contents).expect("Json is not correctly formatted.");
    let mut i = 0;

    assert_eq!(log.worklogs[26].author.displayName, "Steinar Overbeck Cook");

    for e in &log.worklogs {
        println!("{} {} {}", i, e.author.displayName, e.timeSpent);
        i += 1;
        if i > 100 {
            break;
        }
    }
    println!("{}", log.worklogs[0].author.displayName );

}

#[test]
fn test_parse_date() {
    let _dt = match DateTime::parse_from_str("2022-02-04T16:22:28.554+0100", "%Y-%m-%dT%H:%M:%S%.f%z"){
        Ok(dt) => {
            println!("Parsed: {:?}", dt);
            assert_eq!(dt.date_naive() ,NaiveDate::from_ymd_opt(2022,2,4).unwrap());
            assert_eq!(dt.year(), 2022);
            assert_eq!(dt.month(), 02);
            assert_eq!(dt.day(), 4);
            assert_eq!(dt.hour(), 16);
            assert_eq!(dt.minute(), 22);
        },
        Err(err) => panic!("Parsing error {}", err)
    };
}

#[test]
fn test_parse() {
    let json = r#"{"startAt":0,"maxResults":1,"total":8884,"worklogs":[{"self":"https://autostore.atlassian.net/rest/api/2/issue/85002/worklog/129875","author":{"self":"https://autostore.atlassian.net/rest/api/2/user?accountId=557058%3A189520f0-d1fb-4a0d-b555-bc44ec1f4ebc","accountId":"557058:189520f0-d1fb-4a0d-b555-bc44ec1f4ebc","emailAddress":"borge.bekken@autostoresystem.com","avatarUrls":{"48x48":"https://secure.gravatar.com/avatar/0c67157f18660008baae96b0a2e40a61?d=https%3A%2F%2Favatar-management--avatars.us-west-2.prod.public.atl-paas.net%2Finitials%2FBB-1.png","24x24":"https://secure.gravatar.com/avatar/0c67157f18660008baae96b0a2e40a61?d=https%3A%2F%2Favatar-management--avatars.us-west-2.prod.public.atl-paas.net%2Finitials%2FBB-1.png","16x16":"https://secure.gravatar.com/avatar/0c67157f18660008baae96b0a2e40a61?d=https%3A%2F%2Favatar-management--avatars.us-west-2.prod.public.atl-paas.net%2Finitials%2FBB-1.png","32x32":"https://secure.gravatar.com/avatar/0c67157f18660008baae96b0a2e40a61?d=https%3A%2F%2Favatar-management--avatars.us-west-2.prod.public.atl-paas.net%2Finitials%2FBB-1.png"},"displayName":"Børge Bekken","active":true,"timeZone":"Europe/Oslo","accountType":"atlassian"},"updateAuthor":{"self":"https://autostore.atlassian.net/rest/api/2/user?accountId=557058%3A189520f0-d1fb-4a0d-b555-bc44ec1f4ebc","accountId":"557058:189520f0-d1fb-4a0d-b555-bc44ec1f4ebc","emailAddress":"borge.bekken@autostoresystem.com","avatarUrls":{"48x48":"https://secure.gravatar.com/avatar/0c67157f18660008baae96b0a2e40a61?d=https%3A%2F%2Favatar-management--avatars.us-west-2.prod.public.atl-paas.net%2Finitials%2FBB-1.png","24x24":"https://secure.gravatar.com/avatar/0c67157f18660008baae96b0a2e40a61?d=https%3A%2F%2Favatar-management--avatars.us-west-2.prod.public.atl-paas.net%2Finitials%2FBB-1.png","16x16":"https://secure.gravatar.com/avatar/0c67157f18660008baae96b0a2e40a61?d=https%3A%2F%2Favatar-management--avatars.us-west-2.prod.public.atl-paas.net%2Finitials%2FBB-1.png","32x32":"https://secure.gravatar.com/avatar/0c67157f18660008baae96b0a2e40a61?d=https%3A%2F%2Favatar-management--avatars.us-west-2.prod.public.atl-paas.net%2Finitials%2FBB-1.png"},"displayName":"Børge Bekken","active":true,"timeZone":"Europe/Oslo","accountType":"atlassian"},"created":"2022-02-04T16:22:28.554+0100","updated":"2022-02-04T16:22:44.384+0100","started":"2022-01-24T09:00:00.000+0100","timeSpent":"1d","timeSpentSeconds":27000,"id":"129875","issueId":"85002"}]}"#;

    let result = serde_json::from_str::<WorklogsPage>(&json).unwrap();
    assert_eq!("Børge Bekken", result.worklogs[0].author.displayName);

    let datetime = DateTime::parse_from_str("2022-02-04T16:22:28.554+0100", "%Y-%m-%dT%H:%M:%S%.f%#z").unwrap();
    assert_eq!(result.worklogs[0].created, datetime.with_timezone(&Utc));
}

#[test]
fn test_base64() {
    let user = "steinar.cook@autostoresystem.com";
    let token = "vbFYbxdSeahS7KED9sK401E3";
    let mut s: String = String::from(user);
    s.push_str(":");
    s.push_str(token);
    let b64 = base64::encode(s.as_bytes());
    assert_eq!(b64, "c3RlaW5hci5jb29rQGF1dG9zdG9yZXN5c3RlbS5jb206dmJGWWJ4ZFNlYWhTN0tFRDlzSzQwMUUz");
}

#[test]
fn test_date_time() {
    let s = r#"
  {
    "id": "129875",
    "author":  {
        "accountId": "557058:189520f0-d1fb-4a0d-b555-bc44ec1f4ebc",
        "emailAddress": "borge.bekken@autostoresystem.com",
        "displayName": "Børge Bekken"
    },
    "created": "2022-02-04T16:22:28.554+0100",
    "updated": "2022-02-04T16:22:44.384+0100",
    "started": "2022-01-24T09:00:00.000+0100",
    "timeSpent": "1d",
    "timeSpentSeconds": 27000,
    "issueId": "85002"
    }
"#;

    let result = serde_json::from_str::<Worklog>(&s).unwrap();

}