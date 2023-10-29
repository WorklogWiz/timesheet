
# Jira Time Sheet

Rust learning project to extract and update hours logged in Jira.

 * `jira_dbms` - library that manages the Postgres SQL database that Kristian Nessa created for me.
 * `jira_lib` - library with various functions to retrieve data from Jira
 * `jira_worklog` - command line utility to register logged hours into Jira
 * `jira_worklog_etl` - command line utility to extract all Jira worklogs for all issues for all projects not marked as private and shove them 
   and shove them into the Postgresql database


## How to build on MacOS

The script `build.sh` will compile all the binaries and upload them
to the OneDrive directory.

## Cross compiling to Windows on MacOS
How to cross compile from Mac to Windows:
```shell
brew install mingw-w64
rustup target add x86_64-pc-windows-gnu
cargo build --target x86_64-pc-windows-gnu
```

**Note** This was the first entry I found on Google in June 2023. The Windows executable 
is rather large, so perhaps there is better way to do this.